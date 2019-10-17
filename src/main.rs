// vim: set ai et ts=4 sts=4 sw=4:
mod util;
mod puzzle;
mod grid;
mod row;

use std::vec::Vec;
use yaml_rust::{YamlLoader, Yaml};
use self::puzzle::Puzzle;

fn main() {
    let s = "
rows:
    - 5
    - 1 4
    - 1 1 1
    - 1 1 1 1
    - 1 1 1 1
    - 1 1 3 1
    - 1 1 1
    - 1 1 1
    - 3 4 1
    - 3 3
cols:
    - 8
    - 1 1
    - 1 1 5
    - 1 1
    - 1 2 2
    - 2 1 1
    - 5 1
    - 1 2
    - 1 1
    - 8
";
    // note: column numbers are listed top to bottom
    let docs: Vec<Yaml> = YamlLoader::load_from_str(s).unwrap();
    let doc: &Yaml = &docs[0];

    let mut puzzle = Puzzle::from_yaml(doc);
    puzzle.solve();
    println!("{:#?}", puzzle);
    println!("\n{}", puzzle);
}
