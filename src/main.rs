// vim: set ai et ts=4 sts=4 sw=4:
#![allow(dead_code, unused_imports)]
mod util;
mod puzzle;
mod grid;
mod row;

use std::fs;
use std::env;
use std::process::exit;
use std::vec::Vec;
use yaml_rust::{YamlLoader, Yaml};
use self::puzzle::Puzzle;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Missing argument: filename");
        exit(1);
    }
    let contents = fs::read_to_string(&args[1]).expect("Failed to read input file");

    // note: column numbers are listed top to bottom
    let docs: Vec<Yaml> = YamlLoader::load_from_str(&contents).unwrap();
    let doc: &Yaml = &docs[0];

    let mut puzzle = Puzzle::from_yaml(doc);
    if let Err(x) = puzzle.solve() {
        println!("\nFailed to solve puzzle!\n  {}", x);
    }
}
