use std::{fs::File, io::BufReader};

use serde_json::Value;

fn main() {
    let file = File::open("ALPRs.geojson").unwrap();
    let reader = BufReader::new(file);

    let value: Value = serde_json::from_reader(reader).unwrap();
    // extract actuall features
    let h = &value["features"];
    // from here clean and extract!
    let first_node = &h[0];
    // for geomerty just average them ig?
    // could use posGIS stuff but probably a bad idea and complex so we'll sum and get average


    println!("{:?}", first_node);
}
