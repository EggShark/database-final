use sqlx::{query, Connection, PgConnection, Row};
use tokio::runtime::Runtime;

use serde_json::value::Value;


use std::collections::HashSet;
use std::env;
use std::io::{BufReader, Write};
use std::io;
use std::fs::File;

const CREDS: &str = include_str!("../../secret");
const CAMERA_FOV: f64 = 120.0001;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = &args[0];
    let arg_len = args.len();

    if arg_len < 2 {
        println!("please provide an option for using this software");
        usage(path);
        return;
    }
    
    let mut args = args.into_iter().peekable();
    let path = args.next().unwrap();

    let mut operation: Option<Opertation> = None;
    let mut flock_only = false;

    let rt = tokio::runtime::Runtime::new().unwrap();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-v" | "--v" => {
                let long = if let Some(v) = args.peek() {
                    match v.parse() {
                        Ok(v) => v,
                        Err(_) => {
                            println!("Exected validing floating point for longitude");
                            usage(&path);
                            return;
                        }
                    }
                } else {
                    println!("Must be given two points for {} flag", arg);
                    usage(&path);
                    return;
                };
                args.next().unwrap();
                let lat = if let Some(v) = args.peek() {
                    match v.parse() {
                        Ok(v) => v,
                        Err(_) => {
                            println!("Expected valid floating point for latitude");
                            usage(&path);
                            return;
                        }
                    }
                } else {
                    println!("Must be given two points for {} flag", arg);
                    usage(&path);
                    return
                };
                args.next().unwrap();
                operation = Some(Opertation::Visibile(lat, long));
            }
            "-p" | "--p" => {
                let mut path = String::new();
                if let Some(peeked_path) = args.next() {
                    path = peeked_path;
                } else {
                    println!("Must be given file path for {} flag", arg);
                }
                operation = Some(Opertation::Path(path));
            }
            "-f" | "--flock" => flock_only = true,
            "-h" | "--help" => {
                usage(&path);
                return;
            },
            "-d" | "--delete" => {
                let id = if let Some(peeked_id) = args.peek() {
                    match peeked_id.parse() {
                        Ok(v) => v,
                        Err(_) => {
                            println!("Must be given an valid node id for deletetion");
                            usage(&path);
                            return
                        }
                    }
                } else {
                    println!("Must be given an node id to delete");
                    usage(&path);
                    return;
                };
                args.next().unwrap();
                operation = Some(Opertation::Delete(id));
            },
            "-e" | "--edit" => {
                let id = if let Some(peeked_id) = args.peek() {
                    match peeked_id.parse() {
                        Ok(v) => v,
                        Err(_) => {
                            println!("Must be given an valid node id for editing");
                            usage(&path);
                            return
                        }
                    }
                } else {
                    println!("Must be given an node id to edit");
                    usage(&path);
                    return;
                };
                args.next().unwrap();
                operation = Some(Opertation::Edit(id));

            }
            a => {
                println!("Unknown argument of {} found", a);
                usage(&path);
                return;
            }
        }
    }

    if operation.is_none() {
        println!("Some usage must be specified!");
        usage(&path);
        return;
    }

    match operation.unwrap() {
        Opertation::Visibile(lat, long) => visible_search(lat, long, flock_only, rt),
        Opertation::Path(val) => path_visibile_search(&val, rt, flock_only),
        Opertation::Delete(val) => delete_node(val, rt),
        Opertation::Edit(val) => edit_node(val, rt),
        _ => todo!()
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Opertation {
    Path(String),
    Visibile(f64, f64),
    Edit(i64),
    // for sqlx data typing but in reality its always going to be postive
    Delete(i64),
    Create,
}

fn usage(path: &str) {
    println!("{} [--flags]
    At least 1 flag is required!
    -v, --visible <long> <lat>: Will tell you the closest ALPR and if you are likely visible or not
        -f, --flock: Additional flag to toggle only searching for Flock Saftey cameras
    -p, --path <path>: Give a route downloaded from OSM in geoJSON it will tell you what ALPRs will spot you
        -f, --flock: Additional flag to toggle only search for Flock Saftey camers
    -e, --edit: Will start the prompting to enter information for a new ALPR
    -d, --delete id: Deletes the ALPR with the sepcificed id
    -c, --create: Will start the prompting to create a new camera
    -h, --help: prints out the help screen", path)
}

async fn get_database_connection() -> PgConnection {
    let (user_name, password) = CREDS.split_once('\n').unwrap();
    let url = format!("postgresql://{}:{}@ada.mines.edu/csci403", user_name, password.trim());
    let mut conn = PgConnection::connect(&url).await.unwrap();
    sqlx::query("SET search_path TO group120800, public").execute(&mut conn).await.unwrap();
    conn
}

fn path_visibile_search(path: &str, rt: Runtime, flock_only: bool) {
    let file = match File::open(path) {
        Ok(v) => v,
        Err(_) => {
            println!("Error: Was not a valid file path");
            return;
        }
    };
    let reader = BufReader::new(file);
    let value: Value = match serde_json::from_reader(reader) {
        Ok(v) => v,
        Err(_) => {
            println!("File at {} was not valid geoJSON data", path);
            return;
        }
    };

    let geomerty = match value.get("geometry") {
        Some(g) => g,
        None => {
            println!("Exepcted geometry tag in geoJSON data");
            return;
        }
    };

    match geomerty.get("type") {
        Some(Value::String(s)) => {
            match s.as_str() {
                "LineString" => {},
                _ => {
                    println!("Expected LinsString geomerty");
                    return;
                }
            }
        },
        _ => {
            println!("Expected LineString geomerty");
            return;
        }
    }

    let coord_list = match geomerty.get("coordinates") {
        Some(Value::Array(v)) => v,
        _ => {
            println!("Expected coordinate array");
            return;
        }
    };

    let mut visble_nodes: HashSet<i64> = HashSet::new();

    let mut conn = rt.block_on(get_database_connection());
    let query_str = format!("SELECT node_id, ST_Distance(
                ST_Transform(ST_SetSRID(ST_MakePoint($1, $2),4326), 3857),
                ST_Transform(position::geometry, 3857)
            ) * cosd(39.746179) as distance,
            DEGREES(ST_Azimuth(position::geometry, ST_SetSRID(ST_MakePoint($1, $2), 4326))) as headning
            FROM alpr
            WHERE ST_Distance(
                ST_Transform(ST_SetSRID(ST_MakePoint($1, $2),4326), 3857),
                ST_Transform(position::geometry, 3857)
            ) * cosd(39.746179)  < 100 {} 
            ORDER BY ST_Distance(
                ST_Transform(ST_SetSRID(ST_MakePoint($1, $2),4326), 3857),
                ST_Transform(position::geometry, 3857)
            ) * cosd(39.746179) LIMIT 1", if flock_only {"AND manufacturer = 'Flock Saftey'"} else {""});
    for value in coord_list {
        let long = match value[0].as_f64() {
            Some(v) => v,
            None => {
                println!("Expected latitute in coordinates");
                return;
            }
        };
        let lat = match value[1].as_f64() {
            Some(v) => v,
            None => {
                println!("Expected longitude in coordinates");
                return;
            }
        };
        let query = async {
            sqlx::query(&query_str)
                .bind(long)
                .bind(lat)
                .fetch_one(&mut conn)
                .await
        };
        let query_res = match rt.block_on(query) {
            Ok(res) => res,
            Err(sqlx::Error::RowNotFound) => continue,
            Err(e) => {
                println!("UH OG {:?}", e);
                return;
            }
        };
        let node_id: i64 = query_res.get(0);
        let heading: f64 = query_res.get(2);
        if !visble_nodes.contains(&node_id) {
            let query  = async {
                sqlx::query("SELECT direction FROM alpr as a JOIN alpr_direction USING(node_id) WHERE a.node_id = $1")
                    .bind(node_id)
                    .fetch_all(&mut conn)
                .   await
            };

            let dir_res = match rt.block_on(query) {
                Ok(v) => v,
                Err(e) => {
                    println!("UH OG {:?}", e);
                    return;
                }
            };

            if dir_res.is_empty() {
                // take the conservative approach to where if direction isn't listed then its 360
                // vision
                visble_nodes.insert(node_id);
                continue;
            }

            for row in dir_res {
                let pointing_in: i32 = row.get(0);
                let d1 = (heading - pointing_in as f64).abs();
                let d2 = (heading - (360 + pointing_in) as f64).abs();
                if d1 <= CAMERA_FOV || d2 <= CAMERA_FOV {
                    visble_nodes.insert(node_id);
                    break;
                }
            }
        }
    }
    println!("On your path you will be visible to {} cameras", visble_nodes.len());
    print!("Would you like to view these cameras information (Y/n): ");
    let stdin = io::stdin();
    io::stdout().flush().ok();
    let mut input = String::new();
    loop {
        stdin.read_line(&mut input).expect("Could not read from stdin");
        match input.trim() {
            "y" | "Y" | "" => break,
            "n" | "N"  => return,
            v => {
                println!("Input of \"{}\" not valid", v);
                print!("Would you like to view these camera's information (Y/n): ");
                io::stdout().flush().ok();
                input.clear();
            }
        }
    }

    if visble_nodes.is_empty() {
        println!("Nothing to elaborate on!");
        return;
    }
    println!("Camera information is:");
    let mut second_query_str = String::from("SELECT node_id, ST_AsText(position), manufacturer, operator, surviellance_zone from alpr WHERE node_id IN (");
    for node_id in visble_nodes {
        second_query_str.push_str(&node_id.to_string());
        second_query_str.push(',');
    }
    // remove trailing comma cause sql :(
    second_query_str.pop();
    second_query_str.push(')');

    let info_query = async {
        sqlx::query(&second_query_str)
            .fetch_all(&mut conn)
            .await
    };
    let info_query_rs = match rt.block_on(info_query) {
        Ok(v) => v,
        Err(e) => {
            println!("Error with getting camera info {:?}", e);
            return;
        }
    };

    for row in info_query_rs {
        let id: i64 = row.get(0);
        let pos_str: &str = row.get(1);
        let manufacturer: &str = row.get(2);
        let operator: Option<&str> = row.get(3);
        let surviellance_zone: Option<&str> = row.get(4);
        print!("You were seen by camera {}, located at {} made by: {}", id, pos_str, manufacturer);
        if let Some(op) = operator {
            print!(" operated by {}", op);
        }
        if let Some(zone) = surviellance_zone {
            print!(" to survey {} zones", zone);
        }
        println!();
    }
}

fn visible_search(lat: f64, long: f64, flock_only: bool, rt: Runtime) {
    let mut conn = rt.block_on(get_database_connection());
    let query_str = format!("SELECT node_id, surviellance_zone, manufacturer, operator, ST_X(position::geometry) as long, ST_Y(position::geometry) as lat,
            ST_Distance(
                ST_Transform(ST_SetSRID(ST_MakePoint($1, $2),4326), 3857),
                ST_Transform(position::geometry, 3857)
            ) * cosd(39.746179) as distance,
            DEGREES(ST_Azimuth(position::geometry, ST_SetSRID(ST_MakePoint($1, $2), 4326)))
            {} FROM alpr ORDER BY 
            ST_Distance(
                ST_SetSRID(ST_MakePoint($1, $2),4326),
                position
            )
            LIMIT 1;", if flock_only {"WHERE manufacturer = 'Flock Saftey'"} else {""});
    let query = async {
        sqlx::query(&query_str)
        .bind(long)
        .bind(lat)
        .fetch_one(&mut conn)
        .await
        };
    let query_res = rt.block_on(query).unwrap();
    println!("{:?}", query_res);
    let id: i64 = query_res.get(0);
    let surviellance_zone: Option<&str> = query_res.get(1);
    let manufacturer: &str = query_res.get(2);
    let operator: Option<&str> = query_res.get(3);
    let long: f64 = query_res.get(4);
    let lat: f64 = query_res.get(5);
    let distance_meters: f64 = query_res.get(6);
    let heading: f64 = query_res.get(7);

    let mut visible = false;
    if distance_meters <= 100.0 {
        let query = async {
            sqlx::query("SELECT direction FROM alpr as a JOIN alpr_direction USING(node_id) WHERE a.node_id = $1")
                .bind(id)
                .fetch_all(&mut conn)
                .await
                .unwrap()
        };
        let query_res = rt.block_on(query);
        for row in query_res {
            let direction: i32 = row.get(0);
            // assuming 90 degree FOV
            let diffrence = (heading - direction as f64).abs();
            let diffrence2 = (heading - (direction + 360) as f64).abs();
            if diffrence <= CAMERA_FOV || diffrence2 <= CAMERA_FOV {
                visible = true;
                break;
            }
        }
    }

    print!("The closest camera to you is located at ({}, {}) made by {},", lat, long, manufacturer);
    if let Some(zone) = surviellance_zone {
        print!(" to survey {} areas", zone);
    }

    if let Some(operators) = operator {
        print!(" its operated by {}", operators);
    }

    print!("\nyou are {:.2}m away from the camera", distance_meters);
    if visible {
        println!("\nYOU ARE LIKELY VISIBLE TO THIS CAMERA");
    } else {
        println!("\nIt is unlikely that you are visible");
    }
}

fn delete_node(node_id: i64, rt: Runtime) {
    let mut conn = rt.block_on(get_database_connection());
    // start transaction
    rt.block_on(async {
        sqlx::query("BEGIN").execute(&mut conn).await.unwrap();
    });
    let delete = async {
        sqlx::query("DELETE FROM alpr WHERE node_id = $1").bind(node_id).execute(&mut conn).await
    };

    

    match rt.block_on(delete) {
        Ok(_) => {},
        Err(e) => {
            println!("Unable to delete node with id {} because {}", node_id, e);
            rt.block_on(async {sqlx::query("ROLLBACK").execute(&mut conn).await.unwrap()});
        }
    };

    let delete_dirs = async {
        sqlx::query("DELETE FROM alpr_direction WHERE node_id = $1").bind(node_id).execute(&mut conn).await
    };

    match rt.block_on(delete_dirs) {
        Ok(_) => {},
        Err(e) => {
            println!("Unable to delete node with id {}, because {}", node_id, e);
            rt.block_on(async {sqlx::query("ROLLBACK").execute(&mut conn).await.unwrap()});
        }
    }

    print!("Are you sure you want to delete node with id {}, (y/N): ", node_id);
    let stdin = io::stdin();
    io::stdout().flush().ok();
    let mut input = String::new();
    loop {
        stdin.read_line(&mut input).expect("Could not read from stdin");
        match input.trim() {
            "y" | "Y"  => break,
            "n" | "N" | "" => {
                rt.block_on(async {
                    sqlx::query("ROLLBACK").execute(&mut conn).await.unwrap();
                });
                println!("Rolled back change");
                return;
            },
            v => {
                println!("Input of \"{}\" not valid", v);
                print!("Are you sure you want to delete node with id {} (y/N): ", node_id);
                io::stdout().flush().ok();
                input.clear();
            }
        }
    }

    // dont want this actually edditing the data base while still developing
    // will change 
    rt.block_on(async {
        sqlx::query("ROLLBACK").execute(&mut conn).await.unwrap();
    });
    println!("Removed camera with node id {} from the data base", node_id);
}

fn create_node(rt: Runtime) {
    let mut conn = rt.block_on(get_database_connection());
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut node_id_str = String::new();
    print!("Enter the node_id (NOT OPTIONAL): ");
    stdout.flush().ok();
    stdin.read_line(&mut node_id_str).expect("Could not read from stdin");
    let node_id = match node_id_str.trim().parse::<i64>() {
        Ok(v) => v,
        Err(_) => {
            println!("node id must be a valid integer");
            return;
        }
    };

    let mut manufacturer = String::new();
    print!("Enter the manufacturer of the camera (NOT OPTIONAL): ");
    stdout.flush().ok();
    stdin.read_line(&mut manufacturer).expect("Could not read from stdin");
    
    if manufacturer.trim().is_empty() {
        println!("manufacturer is not an optional field");
        return;
    }

    let mut long_str = String::new();
    print!("Enter the longitude of the camera in WGS84 (NOT OPTIONAL): ");
    stdout.flush().ok();
    stdin.read_line(&mut long_str).expect("Could not read from stdin");
    let long = match long_str.trim().parse::<f64>() {
        Ok(v) => v,
        Err(_) => {
            println!("Longitude must be a valid floating point number");
            return;
        }
    };

    let mut lat_str = String::new();
    print!("Enter the latitude of the camera in WGS84 (NOT OPTIONAL): ");
    stdout.flush().ok();
    stdin.read_line(&mut lat_str).expect("Could not read from stdin");
    let lat = match lat_str.trim().parse::<f64>() {
        Ok(v) => v,
        Err(_) => {
            println!("Latitude must be a valid floating point number");
            return;
        }
    };

   let mut operator_str = String::new();
   print!("Enter the operator of the camera (leave blank for NULL): ");
   stdout.flush().ok();
   stdin.read_line(&mut operator_str).expect("Could not read from stdin");
   let operator = match operator_str.trim() {
       "" => None,
       v => Some(v)
   };

   let mut surviellance_zone_str = String::new();
   print!("Enter the type of zone the camera surveys (leave blank for NULL): ");
   stdout.flush().ok();
   stdin.read_line(&mut surviellance_zone_str).expect("Could not read from stdin");
   let surviellance_zone = match surviellance_zone_str.trim() {
       "" => None,
       v => Some(v)
   };

   let mut directions_str = String::new();
   print!("Enter the directions the camera points seperated by commas: ");
   stdout.flush().ok();
   stdin.read_line(&mut directions_str).expect("Could not read from stdin");

   let directions = directions_str
        .trim()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|d| (d.trim().parse::<i64>().expect("Requires valid integers for directions") % 360).abs()).collect::<Vec<i64>>();

   // insert stuff
   let insert_query = async {
       sqlx::query("INSERT INTO ALPR (node_id, manufacturer, operator, surveillance_type, surviellance_zone, position) VALUES (
               $1,
               $2,
               $3,
               'alpr',
               $4,
               ST_SetSRID(ST_MakePoint($5, $6),4326)
           )")
           .bind(node_id)
           .bind(manufacturer)
           .bind(operator)
           .bind(surviellance_zone)
           .bind(long)
           .bind(lat)
           .execute(&mut conn)
           .await
   };

   match rt.block_on(insert_query) {
       Ok(_) => {},
       Err(e) => {
           println!("Falied to insert the node because {}", e);
           rt.block_on(async {sqlx::query("ROLLBACK").execute(&mut conn).await.unwrap()});
           return;
       }
   }

   let dir_str = "INSERT INTO alpr_direction (node_id, direction) VALUES ($1, $2)";
   for dir in directions {
        let dir_insert = async {
            sqlx::query(dir_str)
                .bind(node_id)
                .bind(dir)
                .execute(&mut conn)
                .await
        };
        match rt.block_on(dir_insert) {
            Ok(_) => {},
            Err(e) => {
                println!("Failed to insert the camera direction because {}", e);
                rt.block_on(async {sqlx::query("ROLLBACK").execute(&mut conn).await.unwrap()});
                return;
            }
        }
   }

   
   print!("Are you sure you want to insert node with id {}, (y/N): ", node_id);
   let stdin = io::stdin();
   io::stdout().flush().ok();
   let mut input = String::new();
   loop {
        stdin.read_line(&mut input).expect("Could not read from stdin");
        match input.trim() {
            "y" | "Y"  => break,
            "n" | "N" | "" => {
                rt.block_on(async {
                    sqlx::query("ROLLBACK").execute(&mut conn).await.unwrap();
                });
                println!("Rolled back change");
                return;
            },
            v => {
                println!("Input of \"{}\" not valid", v);
                print!("Are you sure you want to insert node with id {} (y/N): ", node_id);
                io::stdout().flush().ok();
                input.clear();
            }
        }
    }

   rt.block_on(async {
        sqlx::query("ROLLBACK").execute(&mut conn).await.unwrap();
   });
   println!("Node sucessfully inserted!");
}

fn edit_node(node_id: i64, rt: Runtime) {
    let mut conn = rt.block_on(get_database_connection());
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    print!("Enter what you'd like to set the manufacturer to (leave blank to leave unchanged): ");
    stdout.flush().ok();
    let mut manufacturer = String::new();
    stdin.read_line(&mut manufacturer).expect("Could not read from stdin");

    print!("Enter what you'd like to set the operator to (leave blank to leave unchaged): ");
    stdout.flush().ok();
    let mut operator = String::new();
    stdin.read_line(&mut operator).expect("Could not read from stdin");

    print!("Enter what you'd like to set the surviellance_zone to be (leave blank to leave unchanged): ");
    stdout.flush().ok();
    let mut surviellance_zone = String::new();
    stdin.read_line(&mut surviellance_zone).expect("Could not read from stdin");

    print!("Enter what you'd like to set the pointing directions to be (seperated by commas): ");
    stdout.flush().ok();
    let mut directions = String::new();
    stdin.read_line(&mut directions).expect("Could not read from stdin");

    let directions = directions
        .trim()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|d| (d.trim().parse::<i64>().expect("Requires valid integers for directions") % 360).abs()).collect::<Vec<i64>>();

    let roll_back_query = sqlx::query("ROLLBACK");

    rt.block_on(async {sqlx::query("BEGIN").execute(&mut conn).await.unwrap()});
    if !manufacturer.trim().is_empty() {
        let query = async {
            sqlx::query("UPDATE alpr SET manufacturer = $1 WHERE node_id = $2")
                .bind(manufacturer.trim())
                .bind(node_id)
                .execute(&mut conn)
                .await
        };

        match rt.block_on(query) {
            Ok(_) => {},
            Err(e) => {
                println!("Failed to updated maunfacturer because {}, rolling back and aborting", e);
                rt.block_on(roll_back_query.execute(&mut conn)).unwrap();
                return;
            }
        }
    }

    if !operator.trim().is_empty() {
        let query = async {
            sqlx::query("UPDATE alpr SET operator = $1 WHERE node_id = $2")
                .bind(operator.trim())
                .bind(node_id)
                .execute(&mut conn)
                .await
        };

        match rt.block_on(query) {
            Ok(_) => {},
            Err(e) => {
                println!("Failed to update operator because {}, rolling back and aborting", e);
                rt.block_on(roll_back_query.execute(&mut conn)).unwrap();
                return;
            }
        }
    }

    if !surviellance_zone.trim().is_empty() {
        let query = async {
            sqlx::query("UPDATE alpr SET surviellance_zone = $1 WHERE node_id = $2")
                .bind(surviellance_zone.trim())
                .bind(node_id)
                .execute(&mut conn)
                .await
        };

        match rt.block_on(query) {
            Ok(_) => {},
            Err(e) => {
                println!("Failed to updated surviellance zone becuase {} rolling back and aborting", e);
                rt.block_on(roll_back_query.execute(&mut conn)).unwrap();
                return;
            }
        }
    }
    let direction_text = "INSERT INTO alpr_direction (node_id, direction) VALUES ($1, $2)";
    
    if !directions.is_empty() {
        // Delete all directions first
        let delete_query = async {
            sqlx::query("DELETE FROM alpr_direction WHERE node_id = $1").bind(node_id).execute(&mut conn).await
        };

        match rt.block_on(delete_query) {
            Ok(_) => {}
            Err(e) => {
                println!("Failed to updated operator because {}, rolling back and aborting", e);
                rt.block_on(roll_back_query.execute(&mut conn)).unwrap();
                return;
            }
        }

        for dir in directions {
            let query = async {
                sqlx::query(direction_text).bind(node_id).bind(dir).execute(&mut conn).await
            };

            match rt.block_on(query) {
                Ok(_) => {},
                Err(e) => {
                    println!("Failed to insert into directions due to {} rolling back and aborting", e);
                    rt.block_on(roll_back_query.execute(&mut conn)).unwrap();
                    return;
                }
            }
        }
    }

    print!("Are you sure you want to edit node with id {}, (y/N): ", node_id);
    let stdin = io::stdin();
    io::stdout().flush().ok();
    let mut input = String::new();
    loop {
        stdin.read_line(&mut input).expect("Could not read from stdin");
        match input.trim() {
            "y" | "Y"  => break,
            "n" | "N" | "" => {
                rt.block_on(async {
                    sqlx::query("ROLLBACK").execute(&mut conn).await.unwrap();
                });
                println!("Rolled back change");
                return;
            },
            v => {
                println!("Input of \"{}\" not valid", v);
                print!("Are you sure you want to edit node with id {} (y/N): ", node_id);
                io::stdout().flush().ok();
                input.clear();
            }
        }
    }

    rt.block_on(async {
        sqlx::query("ROLLBACK").execute(&mut conn).await.unwrap();
    });
    println!("Removed camera with node id {} from the data base", node_id);
}
