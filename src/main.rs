use std::{fs::File, io::BufReader};
use std::path::Path;

use serde_json::Value;

const CREDS: &str = include_str!("../secret");

#[derive(Debug)]
struct Alpr {
    manufacturer: String,
    id: u64,
    // position in lat long WKID 3857
    position: (f64, f64),
    surveillance_type: String,
    surveillance_zone: Option<String>,
    operator: Option<String>,
    directions: Vec<u64>,
}

fn main() {

    let alprs = extract_alpr_data_from_geojson("ALPRs.geojson");
    println!("processed: {} entries", alprs.len());
    // println!("{:?}", geo[0]);
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
            .parse::<u64>()
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
                        v => v.parse::<f64>().ok().map(|v| v as u64 % 360)
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

        println!("{:?}", alpr);

        alprs.push(alpr);
    }

    alprs
}
