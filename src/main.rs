use std::{fs::File, io::BufReader};
use std::path::Path;

use serde_json::Value;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

// means this is compiled with the secret but uhhh oh well :3
// set search_path TO group121061, public;
const CREDS: &str = include_str!("../secret");

#[derive(Debug)]
struct Alpr {
    manufacturer: String,
    id: i64,
    // position in lat long WKID 3857
    position: (f64, f64),
    surveillance_type: String,
    surveillance_zone: Option<String>,
    operator: Option<String>,
    directions: Vec<i32>,
}

#[tokio::main]
async fn main() {
    let (user_name, password) = CREDS.split_once('\n').unwrap();
    let url = format!("postgresql://{}:{}@ada.mines.edu/csci403", user_name, password);

    let pool = PgPoolOptions::new().connect(&url).await.unwrap();
    let setup = delete_and_create_tables(&pool, user_name);

    let data = extract_alpr_data_from_geojson("ALPRs.geojson");
    setup.await;


    sqlx::query("BEGIN").execute(&pool).await.unwrap();
    for alpr in data.iter() {
        let h = sqlx::query(
           "INSERT INTO ALPR (node_id, manufacturer, operator, surveillance_type, surviellance_zone, position) VALUES (
               $1,
               $2,
               $3,
               $4,
               $5,
               ST_SetSRID(ST_MakePoint($6, $7),4326)
           )"
           )
           .bind(alpr.id)
           .bind(&alpr.manufacturer)
           .bind(&alpr.operator)
           .bind(&alpr.surveillance_type)
           .bind(&alpr.surveillance_zone)
           .bind(alpr.position.0)
           .bind(alpr.position.1)
           .execute(&pool)
           .await;

        match h {
            Ok(_) => {}
            Err(e) => {
                println!("Failed to insernt ALRP with id {} becuase {:?}", alpr.id, e);
                sqlx::query("ROLLBACK").execute(&pool).await.unwrap();
                continue;
            }
        }
        println!("ALRP with id {} inserted!", alpr.id);
    }
    sqlx::query("COMMIT").execute(&pool).await.unwrap();


    let mut failed_dirs = vec![];
    for alpr in data {
        if alpr.directions.is_empty() {
            continue;
        }
        sqlx::query("BEGIN").execute(&pool).await.unwrap();
        for dir in alpr.directions {
            let q2 = sqlx::query("INSERT INTO ALPR_direction (node_id, direction) VALUES ($1, $2)")
                .bind(alpr.id)
                .bind(dir)
                .execute(&pool)
                .await;

            match q2 {
                Ok(_) => {},
                Err(e) => {
                    println!("Failed to insernt ALRP_direction with id {} becuase {:?}", alpr.id, e);
                    sqlx::query("ROLLBACK").execute(&pool).await.unwrap();
                    failed_dirs.push((alpr.id, dir));
                    continue;
                }
            }
        }
        sqlx::query("COMMIT").execute(&pool).await.unwrap();
        println!("ALPR direction, {} inserted", alpr.id);
    }

    println!("failed to insert: {}, retrying now", failed_dirs.len());
    for (id, dir) in failed_dirs {
        sqlx::query("BEGIN").execute(&pool).await.unwrap();
        let q2 = sqlx::query("INSERT INTO ALPR_direction (node_id, direction) VALUES ($1, $2)")
                .bind(id)
                .bind(dir)
                .execute(&pool)
                .await;
        match q2 {
            Ok(_) => {}
            Err(e) => {
                println!("{} failed second chance becuase {:?}", id, e);
                sqlx::query("ROLLBACK").execute(&pool).await.unwrap();
                continue;
            }
        }
        sqlx::query("COMMIT").execute(&pool).await.unwrap();
    }


}

async fn delete_and_create_tables(pool: &PgPool, user_name: &str) {
    let search_path = format!("SET search_path TO {}, public", user_name);

    sqlx::query(&search_path).execute(pool).await.unwrap();
    sqlx::query("DROP TABLE IF EXISTS ALPR_direction").execute(pool).await.unwrap();
    sqlx::query("DROP TABLE IF EXISTS ALPR").execute(pool).await.unwrap();

    sqlx::query("CREATE TABLE ALPR (
            node_id BIGINT NOT NULL,
            manufacturer VARCHAR NOT NULL,
            operator VARCHAR,
            surveillance_type VARCHAR NOT NULL,
            surviellance_zone VARCHAR,
            position geography(POINT) NOT NULL,
            PRIMARY KEY (node_id)
    )").execute(pool).await.unwrap();

    sqlx::query("CREATE TABLE ALPR_direction (
            node_id BIGINT REFERENCES ALPR (node_id),
            direction INTEGER
    )").execute(pool).await.unwrap();
}

fn extract_alpr_data_from_geojson<P: AsRef<Path>>(path: P) -> Vec<Alpr> {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let mut alprs = vec![];
    let value: Value = serde_json::from_reader(reader).unwrap();
    // extract actuall features
    let h = match &value["features"] {
        Value::Array(v) => v,
        _ => unreachable!()
    };
    // from here clean and extract!
    for node in h {
        let properties = &node["properties"];

        // Some will have none maybe just exlude from DB?
        let manu = match properties["manufacturer"].as_str() {
            Some(s) => s,
            None => {
                let h = properties["brand"].as_str();
                if h.is_none() {
                    continue;
                }
                h.unwrap()
            }
        };
        let servey_type = properties["surveillance:type"].as_str().unwrap();
        // optional defualt to road/traffic?
        let survey_zone = properties["surveillance:zone"].as_str().map(|d| d.to_string());
        // need this to be optional typing for saftey
        let operator = properties["operator"].as_str().map(|d| d.to_string());
        let id = properties["@id"]
            .as_str()
            .unwrap()
            .split_once('/')
            .unwrap()
            .1
            .parse::<i64>()
            .unwrap();
        let geo = &node["geometry"];
        let geo_type = geo["type"].as_str().unwrap();
        let directions = properties["direction"]
            .as_str()
            .map(|f| f.split(&[';', '-', ','])
                .filter(|s| !s.is_empty())
                .filter_map(|s: &str| {
                    match s.trim() {
                        "n" | "N" => Some(0),
                        "NNE" | "nne" => Some(22),
                        "NE" | "ne" => Some(45),
                        "ENE" | "ene" => Some(67),
                        "e" | "E" => Some(90),
                        "ESE" | "ese" => Some(112),
                        "SE" | "se" => Some(135),
                        "SSE" | "sse" => Some(157),
                        "s" | "S" => Some(180),
                        "SSW" | "ssw" => Some(202),
                        "SW" | "sw" => Some(225),
                        "WSW" | "wsw" => Some(247),
                        "w" | "W" => Some(270),
                        "WNW" | "wnw" => Some(292),
                        "NW" | "nw" => Some(315),
                        "NNW" | "nnw" => Some(337),
                        // all thats left is "Disabled", "forward", "backward", and "Clockwise 180"
                        // Those are relative and require extra knowledge i dont have so we will
                        // just skip them cause lmao
                        v => v.parse::<f64>().ok().map(|v| v as i32 % 360)
                    }
                })
                .collect::<Vec<_>>())
                .unwrap_or_default();

        // get average type as we just dont know yk
        let coords = match geo_type {
            "Point" => {
                let point = geo["coordinates"].as_array().unwrap();
                let lat = point[0].as_f64().unwrap();
                let long = point[1].as_f64().unwrap();
                (lat, long)
            }
            "LineString" => {
                let points = geo["coordinates"].as_array().unwrap();
                let mut average_lat = 0.0;
                let mut average_long = 0.0;
                
                for (idx, point) in points.iter().enumerate() {
                    average_lat += (point[0].as_f64().unwrap() - average_lat)/(idx + 1) as f64;
                    average_long += (point[1].as_f64().unwrap() - average_long)/(idx + 1) as f64;
                }
                
                (average_lat, average_long)
            }
            "Polygon" => {
                let points = geo["coordinates"].as_array().unwrap();
                let points = points[0].as_array().unwrap();
                let mut average_lat = 0.0;
                let mut average_long = 0.0;
                
                for (idx, point) in points.iter().enumerate() {
                    average_lat += (point[0].as_f64().unwrap() - average_lat)/(idx + 1) as f64;
                    average_long += (point[1].as_f64().unwrap() - average_long)/(idx + 1) as f64;
                }
                
                (average_lat, average_long)    
            }
            e => {
                panic!("unknown coords type reached! {}", e)
            }
        };
        let alpr = Alpr {
            manufacturer: manu.to_string(),
            position: coords,
            surveillance_type: servey_type.to_string(),
            surveillance_zone: survey_zone,
            operator,
            directions,
            id,
        };
        alprs.push(alpr);
    }

    alprs
}
