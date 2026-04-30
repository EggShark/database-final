use sqlx::{query, Connection, PgConnection, Row};
use tokio::runtime::Runtime;

use std::env;

const CREDS: &str = include_str!("../../secret");

fn main() {
    println!("Hello, world!");
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
                let (mut long, mut lat) = (0.0, 0.0);
                if let Some(v) = args.peek() {
                    match v.parse() {
                        Ok(v) => long = v,
                        Err(_) => {
                            println!("Exected validing floating point for longitude");
                            usage(&path);
                            return;
                        }
                    }
                }
                args.next().unwrap();
                if let Some(v) = args.peek() {
                    match v.parse() {
                        Ok(v) => lat = v,
                        Err(_) => {
                            println!("Expected valid floating point for latitude");
                            usage(&path);
                            return;
                        }
                    }
                }
                args.next().unwrap();
                operation = Some(Opertation::Visibile(long, lat));
            }
            "-f" | "--flock" => flock_only = true,
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
        Opertation::Visibile(long, lat) => visible_search(long, lat, flock_only, rt),
        _ => todo!()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Opertation {
    Visibile(f64, f64),
    Edit,
    // for sqlx data typing but in reality its always going to be postive
    Delete(i64),
    Create,
    Help,
}

fn usage(path: &str) {
    println!("{} [--flags]
    At least 1 flag is required!
    -v, --visible long lat: Will tell you the closest ALPR and if you are likely visible or not
        -f, --flock: Additional flag to toggle only searching for Flock Saftey cameras
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

fn visible_search(long: f64, lat: f64, flock_only: bool, rt: Runtime) {
    let mut conn = rt.block_on(get_database_connection());
    let query = async {
        sqlx::query("SELECT node_id, surviellance_zone, manufacturer, ST_X(position::geometry) as long, ST_Y(position::geometry) as lat,
            ST_Distance(
                ST_Transform(ST_SetSRID(ST_MakePoint($1, $2),4326), 3857),
                ST_Transform(position::geometry, 3857)
            ) * cosd(39.746179) as distance,
            DEGREES(ST_Azimuth(position::geometry, ST_SetSRID(ST_MakePoint($1, $2), 4326)))
            FROM alpr ORDER BY 
            ST_Distance(
                ST_SetSRID(ST_MakePoint($1, $2),4326),
                position
            )
            LIMIT 1;"
        )
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
    let long: f64 = query_res.get(3);
    let lat: f64 = query_res.get(4);
    let distance_meters: f64 = query_res.get(5);
    let heading: f64 = query_res.get(6);

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
            if diffrence <= 90.0001 || diffrence2 <= 90.0001 {
                visible = true;
                break;
            }
        }
    }

    print!("The closest camera to you is located at ({}, {}) made by {},", lat, long, manufacturer);
    if let Some(zone) = surviellance_zone {
        print!(" to survey {} areas", zone);
    }
    print!("\nyou are {:.2}m away from the camera", distance_meters);
    if visible {
        println!("\nYOU ARE LIKELY VISIBLE TO THIS CAMERA");
    } else {
        println!("\nIt is unlikely that you are visible");
    }
}
