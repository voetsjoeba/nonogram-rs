// vim: set ai et ts=4 sts=4 sw=4:
#![allow(dead_code, unused_imports)]
use std::fs;
use std::io;
use std::env;
use std::ops::Range;
use std::convert::TryFrom;
use std::process::exit;
use std::vec::Vec;
use yaml_rust::{YamlLoader, Yaml};
use clap::{Arg, App, ArgMatches};

mod util;
mod puzzle;
mod grid;
mod row;
mod ui;

use self::util::{is_a_tty, Direction};
use self::puzzle::Puzzle;
use self::ui::ui_main;
use self::grid::{Change, StatusChange, RunChange, SquareStatus};

#[derive(Debug)]
pub struct Args {
    ui: bool,
    input_file: String,
    emit_color: bool,
    visual_groups: Option<usize>,
    actions_on_stall: Vec<Change>, // 'status:row-row,col,new_status'     (new_status=["FilledIn", "CrossedOut"])
                                   // 'status:row,col-col,new_status'
                                   // 'run:row-row,col,new_run_idx'
                                   // 'run:row,col-col,new_run_idx'
}

fn make_action_change(output: &mut Vec<Change>, row: usize, col: usize, direction: Direction, action_type: &str, action_parts: &Vec<String>)
{
    match action_type {
        "status" => {
            let new_status: SquareStatus = SquareStatus::try_from(&*action_parts[2]).unwrap(); // &* to explicitly convert String to &str
            output.push(Change::from(StatusChange::new(row, col, SquareStatus::Unknown, new_status)));
        },
        "run"    => {
            // assigning a run to a square is only possible if it's already filled in, so for
            // convenience we'll automatically insert a FilledIn change as well so that the user
            // doesn't have to remember to add those in manually.
            let new_run: usize = action_parts[2].parse().unwrap();
            output.push(Change::from(StatusChange::new(row, col, SquareStatus::Unknown, SquareStatus::FilledIn)));
            output.push(Change::from(RunChange::new(row, col, direction, None, new_run)));
        },
        _        => panic!("unrecognized action type: {}", action_type),
    }
}

fn parse_actions(actions_str: String) -> Vec<Change> {
    let mut changes = Vec::<Change>::new();

    let actions: Vec<String> = actions_str.split(";").map(|s| s.to_string()).collect();
    for action_str in actions {
        let mut split = action_str.split(":");
        let action_type: &str = split.next().unwrap(); // "run", "status"
        let action_spec: &str = split.next().unwrap(); // remainder

        let action_parts: Vec<String> = action_spec.split(",").map(|s| s.to_string()).collect();
        // are we dealing with a row-row,col or a row,col-col?
        let rows: Vec<usize> = action_parts[0].split("-").map(|s| s.parse::<usize>().unwrap()).collect();
        let cols: Vec<usize> = action_parts[1].split("-").map(|s| s.parse::<usize>().unwrap()).collect();

        if rows.len() == 0 || rows.len() > 2 {
            panic!("bad action spec: row specifier must be either a single row or a row1-row2 range");
        }
        if cols.len() == 0 || cols.len() > 2 {
            panic!("bad action spec: col specifier must be either a single col or a col1-col2 range");
        }
        if rows.len() == cols.len() {
            panic!("bad action spec: exactly one of rows and/or columns must be specified as a range, not both simultaneously, and not neither");
        }

        let rows: Range<usize> = if rows.len() == 1 { rows[0]..rows[0]+1 } else { rows[0]..rows[1]+1 };
        let cols: Range<usize> = if cols.len() == 1 { cols[0]..cols[0]+1 } else { cols[0]..cols[1]+1 };
        let direction: Direction = if rows.len() > 1 { Direction::Vertical } else { Direction::Horizontal };

        match direction {
            Direction::Horizontal => {
                for col in cols.start..cols.end {
                    make_action_change(&mut changes, rows.start, col, direction, action_type, &action_parts);
                }
            },
            Direction::Vertical   => {
                for row in rows.start..rows.end {
                    make_action_change(&mut changes, row, cols.start, direction, action_type, &action_parts);
                }
            },
        }
    }
    changes
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
                   .arg(Arg::with_name("ui")
                             .long("ui")
                             .takes_value(false))
                   .arg(Arg::with_name("groups")
                             .help("row group sizes when outputting puzzle visually")
                             .short("g")
                             .long("groups")
                             .takes_value(true)
                             .required(false)
                             .default_value("5"))
                   .arg(Arg::with_name("actions_on_stall")
                             .help(
r"additional actions to apply when the solver runs out actions to take.
value is a ';'-separated string of action specifiers, which can be formatted as one of:
    status:row,col1-col2,new_status
    status:row1-row2,col,new_status
    run:row,col1-col2,run_index
    run:row1-row2,col,run_index

where 'new_status' is one of 'CrossedOut', 'FilledIn'.
Exactly one of the row or columns must be specified as a range, not both and not neither. Ranges are 0-based and inclusive.
Run assignment actions will automatically fill in squares prior to assigning a run to the square.")
                             .long("on-stall")
                             .takes_value(true))
                   .get_matches();

    let args: Args = Args {
        ui: args.is_present("ui"),
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
        actions_on_stall: match args.value_of("actions_on_stall") {
            Some(x)      => parse_actions(x.to_string()),
            None         => vec![],
        }
    };

    let contents = fs::read_to_string(&args.input_file)
                       .expect("Failed to read input file");

    // note: column numbers are listed top to bottom
    let docs: Vec<Yaml> = YamlLoader::load_from_str(&contents).unwrap();
    let doc: &Yaml = &docs[0];

    let mut puzzle = Puzzle::from_yaml(doc);
    if args.ui {
        if let Err(x) = puzzle.solve(&args) {
            println!("\nFailed to solve puzzle!\n  {}", x);
        }
        ui_main(puzzle, &args);
    } else {
        if let Err(x) = puzzle.solve(&args) {
            println!("\nFailed to solve puzzle!\n  {}", x);
        }
    }
}
