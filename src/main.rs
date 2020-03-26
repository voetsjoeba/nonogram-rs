// vim: set ai et ts=4 sts=4 sw=4:
#![allow(dead_code, unused_imports)]
use std::fs;
use std::mem;
use std::io;
use std::env;
use std::ops::Range;
use std::convert::TryFrom;
use std::process::exit;
use std::vec::Vec;
use yaml_rust::{YamlLoader, Yaml};
use clap::{Arg, App, ArgMatches};
use fern;
use log::{self, trace, debug, info, log_enabled, Level::Debug};

mod util;
mod puzzle;
mod grid;
mod row;
mod ui;

use self::util::{is_a_tty, Direction, Direction::*};
use self::puzzle::{Puzzle, Solver};
use self::row::{Row, DirectionalSequence};
use self::ui::ui_main;
use self::grid::{Change, StatusChange, RunChange, SquareStatus, Error};

#[derive(Debug)]
pub struct Args {
    ui: bool,
    verbosity: u64,
    input_file: String,
    emit_color: bool,
    visual_groups: Option<usize>,
}

fn _solve_with_logic(solver: &mut Solver, args: &Args) -> Result<(), Error>
{
    // tries to solve the puzzle as far as possible using only logically-inferrable changes
    // returns Ok(()) when there are no more actions (regardless of whether the puzzle has been solved),
    // or Err(Error) in case a conflict or impossibility was found.
    while let Some(iteration_result) = solver.next() {
        match iteration_result {
            Ok((row_dir, row_idx, changes)) => {
                if log_enabled!(Debug) {
                    debug!("finished solvers on {} row {}; changes in this iteration:", row_dir, row_idx);
                    for change in &changes {
                        debug!("  {}", change);
                    }

                    debug!("\n{}", solver.puzzle._fmt(args.visual_groups, args.emit_color));
                    debug!("--------------------------------------");
                    debug!("");
                }
            },
            Err(e) => {
                debug!("\nencountered error during solving:");
                debug!("{}", e);
                return Err(e);
            }
        }
    }
    return Ok(())
}

fn solve(puzzle: Puzzle, args: &Args) -> Result<Puzzle, (Error, Puzzle)>
{
    // attempts to solve the given puzzle to completion.
    // returns the solved puzzle on success, or an error indicator in case of an impossibility or a conflict.

    let mut solver = Solver::new(puzzle);
    //let mut speculation_bases = Vec::<Puzzle>::new();

    // keep a queue of rows to be looked at, and run the individual solvers on each
    // of them in sequence until there are none left in the queue. whenever a change
    // is made to a square in the grid, those rows are added back into the queue
    // for evaluation on the next run. completed runs are removed from the queue.
    debug!("starting state:");
    debug!("\n{}", solver.puzzle._fmt(args.visual_groups, args.emit_color));

    loop
    {
        if let Err(e) = _solve_with_logic(&mut solver, args) {
            return Err((e, solver.puzzle));
        }

        debug!("final state:");
        debug!("\n{}", solver.puzzle._fmt(args.visual_groups, args.emit_color));

        if solver.puzzle.is_completed() {
            debug!("puzzle solved! ({} iterations)", solver.iterations);
            break;
        }

        debug!("puzzle partially solved, out of actions ({} iterations).", solver.iterations);

        // we're out of decisions that can be made with logic, so we're forced to start solving
        // speculatively -- i.e. make a decision at some point and see if it introduces a logic error;
        // if it does, revert the work and make the opposite change.
        let edited_puzzle = solver.puzzle.clone();

        // find a square with unknown state and set it to something, and try to continue
        // TODO: how to choose a square to speculatively change, and do we make it filled in or crossed out?
        // can we come up with some metric of "further solving power" resulting from changing a square's state?
        // TODO: besides setting a square's state, we could also pick one that's filled in but doesn't have a known
        // run, and update the run and see what happens; that might actually give pretty good solving power ...
        let mut unknown_square: Option<(usize, usize)> = None;
        let incomplete_rows = edited_puzzle.incomplete_rows();
        for (d,i) in incomplete_rows {
            let row: &Row = solver.puzzle.get_row(d,i);
            if let Some(sq) = (0..row.length).map(|at| row.get_square(at))
                                             .filter(|sq| sq.get_status() == SquareStatus::Unknown)
                                             .next() {
                unknown_square = Some((sq.get_col(), sq.get_row()));
                break;
            }
        }

        // decide that it's gonna be a filled in square and see if anything freaks out
        let (x,y) = unknown_square.unwrap(); // has to succeed, otherwise the puzzle would've been solved
        debug!("speculatively change: setting square (x={}, y={}) to {}", x, y, SquareStatus::FilledIn);
        edited_puzzle.get_square_mut(x,y).set_status(SquareStatus::FilledIn).unwrap();

        // recursively try to solve with the given speculative change; in case of a conflict, make the inverse
        // change and continue.
        match solve(edited_puzzle, args) {
            Ok(solved_puzzle) =>  {
                // we made the right edit, and the recursive call managed to finish solving the whole puzzle,
                // so we can just make that our current one and break out of the solve loop
                solver.puzzle = solved_puzzle;
                break;
            },
            Err(_) => {
                // we made the wrong edit; apply the inverse change and continue trying to solve it
                debug!("speculative change (x={}, y={}) -> {} produced an error", x, y, SquareStatus::FilledIn);
                debug!("must therefore be {} instead, making that change", SquareStatus::CrossedOut);
                solver.puzzle.get_square_mut(x,y).set_status(SquareStatus::CrossedOut).unwrap();
            },
        }
    }
    Ok(solver.puzzle)
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
                   .arg(Arg::with_name("verbose")
                             .help("Increases logging verbosity each use for up to 3 times")
                             .short("v")
                             .long("verbose")
                             .multiple(true))
                   .get_matches();

    let args: Args = Args {
        ui: args.is_present("ui"),
        verbosity: args.occurrences_of("verbose"),
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

    let mut log_config = fern::Dispatch::new()
                            .format(|out, msg, _record| {
                                out.finish(format_args!("{}", msg))
                            })
                            .chain(io::stdout());
    log_config = match args.verbosity {
        0 => log_config.level(log::LevelFilter::Info),
        1 => log_config.level(log::LevelFilter::Debug),
        _ => log_config.level(log::LevelFilter::Trace),
    };
    log_config.apply().unwrap();

    let contents = fs::read_to_string(&args.input_file)
                       .expect("Failed to read input file");

    // note: column numbers are listed top to bottom
    let docs: Vec<Yaml> = YamlLoader::load_from_str(&contents).unwrap();
    let doc: &Yaml = &docs[0];

    let puzzle = Puzzle::from_yaml(doc);
    if args.ui {
        ui_main(puzzle, &args);
    } else {
        match solve(puzzle, &args) {
            Ok(solved) => {
                println!("{}", solved._fmt(args.visual_groups, args.emit_color));
            },
            Err((e, partially_solved)) => {
                println!("{}", partially_solved._fmt(args.visual_groups, args.emit_color));
                println!("encountered error during solving: {}", e);
                debug!("{}", partially_solved.dump_state());
            },
        }
    }
}
