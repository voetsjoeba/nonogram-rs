// vim: set ai et ts=4 sts=4 sw=4:
#![allow(dead_code, unused_imports)]
use std::fs;
use std::io;
use std::env;
use std::convert::TryFrom;
use std::process::exit;
use std::vec::Vec;
use yaml_rust::{YamlLoader, Yaml};
use clap::{Arg, App, ArgMatches};

mod util;
mod puzzle;
mod grid;
mod row;

use self::util::is_a_tty;
use self::puzzle::Puzzle;

pub struct Args {
    input_file: String,
    emit_color: bool,
    visual_groups: Option<usize>,
}

fn main() {
    let args = App::new("nonogram")
                   .arg(Arg::with_name("input_file")
                             .required(true)
                             .help("input YAML file containing the puzzle definition")
                             .index(1))
                   .arg(Arg::with_name("color")
                             .help("whether to output ANSI color escape sequences")
                             .long("color")
                             .required(false)
                             .possible_values(&["yes", "no", "auto"])
                             .default_value("auto"))
                   .arg(Arg::with_name("groups")
                             .help("row group sizes when outputting puzzle visually")
                             .short("g")
                             .long("groups")
                             .takes_value(true)
                             .required(false)
                             .default_value("5"))
                   .get_matches();

    let args: Args = Args {
        input_file: args.value_of("input_file").unwrap().to_string(),
        emit_color: match args.value_of("color") {
            Some("yes")  => true,
            Some("no")   => false,
            _ => is_a_tty(io::stdout()),
        },
        visual_groups: match args.value_of("groups") {
            Some("0")    => None,
            Some(x)      => Some(x.parse::<usize>().unwrap_or(5usize)),
            None         => Some(5usize),
        },
    };

    let contents = fs::read_to_string(&args.input_file)
                       .expect("Failed to read input file");

    // note: column numbers are listed top to bottom
    let docs: Vec<Yaml> = YamlLoader::load_from_str(&contents).unwrap();
    let doc: &Yaml = &docs[0];

    let mut puzzle = Puzzle::from_yaml(doc);
    if let Err(x) = puzzle.solve(&args) {
        println!("\nFailed to solve puzzle!\n  {}", x);
    }
}
