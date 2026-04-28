use std::env;

const CREDS: &str = include_str!("../../secret");

fn main() {
    println!("Hello, world!");
    let args: Vec<String> = env::args().collect();
}
