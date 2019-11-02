// vim: set ai et ts=4 sw=4 sts=4:
use std::fmt;
use std::io;
use std::rc::Rc;
use std::cell::{Ref, RefMut, RefCell};
use std::convert::TryFrom;
use std::collections::{VecDeque, HashSet};
use yaml_rust::Yaml;
use ansi_term::ANSIString;

use super::Args;
use super::grid::{Grid, Square, SquareStatus, Change, Changes, Error, HasGridLocation};
use super::util::{ralign, lalign_colored, ralign_joined_coloreds, Direction, Direction::*, is_a_tty};
use super::row::{Row, Run};

pub struct Solver {
    pub puzzle: Puzzle,
    pub queue: VecDeque<(Direction, usize)>,
    pub iterations: usize,
    pub max_iterations: usize,
}
impl Solver {
    pub fn new(puzzle: Puzzle) -> Self
    {
        let mut queue = VecDeque::<(Direction, usize)>::new();
        queue.extend(puzzle.rows.iter().map(|r| (r.direction, r.index)));
        queue.extend(puzzle.cols.iter().map(|c| (c.direction, c.index)));

        Self {
            puzzle,
            queue,
            iterations: 0,
            max_iterations: 100_000,
        }
    }
    pub fn apply_and_feed_change(&mut self, change: &Change) {
        self.puzzle.apply_change((*change).clone()).expect("");
        self._refeed_change(change);
    }
    fn _refeed_change(&mut self, change: &Change) {
        // takes a change and feeds the row and column that it affected back into the
        // queue.
        let (row, col) = (change.get_row(), change.get_col());
        let h_value = (self.puzzle.rows[row].direction, self.puzzle.rows[row].index);
        let v_value = (self.puzzle.cols[col].direction, self.puzzle.cols[col].index);
        if !self.queue.contains(&v_value) { self.queue.push_back(v_value); }
        if !self.queue.contains(&h_value) { self.queue.push_back(h_value); }
    }
    fn _iter_next(&mut self) -> Option<<Solver as Iterator>::Item>
    {
        macro_rules! changes_or_return {
            ($exp:expr) => {{
                match $exp {
                    Ok(changes) => changes,
                    Err(e)      => return Some(Err(e)),
                }
            }}
        };
        // iterate over the queue and run solver logic on them until some changes are found, and return them;
        // if we're out of rows to investigate, return None.
        while let Some((d,i)) = self.queue.pop_front()
        {
            self.iterations += 1;
            if self.iterations >= self.max_iterations {
                panic!("max iterations exceeded, aborting");
            }

            let row = match d {
                Horizontal => &mut self.puzzle.rows[i],
                Vertical   => &mut self.puzzle.cols[i],
            };

            // before doing any further work, check whether this row is already_completed
            // (includes handling of trivial cases like empty rows etc)
            let mut changes = Vec::<Change>::new();
            changes.extend(changes_or_return!(row.check_completed_runs()));
            changes.extend(changes_or_return!(row.check_completed()));

            if !row.is_completed() {
                row.update_possible_run_placements();
                changes.extend(changes_or_return!(row.infer_run_assignments()));
                changes.extend(changes_or_return!(row.infer_status_assignments()));
            }

            if changes.len() > 0 {
                // found some changes in this row; feed the affected rows and columns
                // back into the queue, and return the changes made.
                for change in &changes {
                    self._refeed_change(change);
                }
                return Some(Ok((d, i, changes)));
            } else {
                // no changes made, try next row in the queue.
            }
        }
        None // out of actions
    }
}
impl Iterator for Solver {
    type Item = Result<(Direction, usize, Changes), Error>; // row direction, index and list of changes applied in this iteration, or an error indicating a problem

    fn next(&mut self) -> Option<Self::Item> {
        self._iter_next()
    }
}

#[derive(Debug)]
pub struct Puzzle {
    pub rows: Vec<Row>,
    pub cols: Vec<Row>,
    pub grid: Rc<RefCell<Grid>>,
}

impl Puzzle {
    pub fn new(grid: &Rc<RefCell<Grid>>,
               row_run_lengths: &Vec<Vec<usize>>,
               col_run_lengths: &Vec<Vec<usize>>) -> Self
    {
        let rows = (0..grid.borrow().height()).map(|y| Row::new(grid, Horizontal, y, &row_run_lengths[y]))
                                              .collect::<Vec<_>>();
        let cols = (0..grid.borrow().width()).map(|x| Row::new(grid, Vertical, x, &col_run_lengths[x]))
                                             .collect::<Vec<_>>();
        Puzzle {
            rows: rows,
            cols: cols,
            grid: Rc::clone(grid),
        }
    }
    pub fn width(&self) -> usize { self.grid.borrow().width() }
    pub fn height(&self) -> usize { self.grid.borrow().height() }

    pub fn from_yaml(doc: &Yaml) -> Puzzle
    {
        let row_run_lengths = Self::_parse_row(&doc["rows"]);
        let col_run_lengths = Self::_parse_row(&doc["cols"]);
        let grid = Rc::new(RefCell::new(
            Grid::new(col_run_lengths.len(), row_run_lengths.len())
        ));
        Puzzle::new(&grid, &row_run_lengths, &col_run_lengths)
    }

    fn _parse_row(input: &Yaml) -> Vec<Vec<usize>> {
		let list: &Vec<Yaml> = input.as_vec().unwrap();
        list.iter()
		    .map(|yaml_val| Self::_parse_row_runs(yaml_val))
			.collect()
    }

    fn _parse_row_runs(input: &Yaml) -> Vec<usize> {
        match input {
            Yaml::String(_)  => { input.as_str().unwrap()
                                       .split_whitespace()
                                       .map(|int| int.trim().parse().unwrap())
                                       .collect()
                                },
            Yaml::Integer(_) => { vec![ usize::try_from(input.as_i64().unwrap()).unwrap() ] }
            Yaml::Null       => { vec![] }
            _ => panic!("Unexpected data type: {:?}", input),
        }
    }

    pub fn get_square(&self, x: usize, y: usize) -> Ref<Square> {
        let grid = self.grid.borrow();
        Ref::map(grid, |g| g.get_square(x, y))
    }
    pub fn get_square_mut(&self, x: usize, y: usize) -> RefMut<Square> {
        let grid = self.grid.borrow_mut();
        RefMut::map(grid, |g| g.get_square_mut(x, y))
    }
    fn apply_change(&mut self, change: Change) -> Result<Option<Change>, Error> {
        let mut square = self.get_square_mut(change.get_col(), change.get_row());
        square.apply_change(change)
    }

    pub fn is_completed(&self) -> bool {
        self.rows.iter().all(|r| r.is_completed()) &&
            self.cols.iter().all(|c| c.is_completed())
    }

}

impl Puzzle {
    pub fn dump_state(&self) {
        println!("run possible placements:");
        for row in self.rows.iter().chain(self.cols.iter()) {
            if row.is_trivially_empty() { continue; }
            println!("  {:-10} row {:2}:", row.direction, row.index);
            for run in &row.runs {
                println!("    run {:2} (len {}): {}", run.index, run.length,
                    run.possible_placements.iter()
                                           .map(|range| format!("[{},{}]", range.start, range.end-1))
                                           .collect::<Vec<_>>()
                                           .join(", "));
            }
        }

        println!("run assignment overview:");
        let grid = self.grid.borrow();
        for y in 0..self.height() {
            for x in 0..self.width() {
                let square: &Square = grid.get_square(x, y);
                if square.get_status() == SquareStatus::FilledIn {
                    println!("  {}: hrun_index={}, vrun_index={}",
                        square.fmt_location(),
                        if let Some(idx) = square.get_run_index(Direction::Horizontal) { idx.to_string() } else { "?".to_string() },
                        if let Some(idx) = square.get_run_index(Direction::Vertical) { idx.to_string() } else { "?".to_string() }
                    );
                }
            }
        }
    }

    // helper functions for Puzzle::fmt
    pub fn _fmt(&self, subdivision: Option<usize>, emit_color: bool)
        -> String
    {
        // if subdivision is given, insert visual subdivisor lines across the grid every Nth row/col
        let row_prefixes: Vec<Vec<ANSIString>> =
            self.rows.iter()
                     .map(|row| row.runs.iter()
                                        .map(|run| run.to_colored_string())
                                        .collect::<Vec<_>>())
                     .collect();

        let prefix_len = row_prefixes.iter()
                                     .map(|parts| parts.iter()
                                                       .fold(0, |sum, ansi_str| sum + ansi_str.len() + 1) // note: .len() returns length WITHOUT ansi color escape sequences
                                                  -1) // minus one at the end to match the length of a join(" ")
                                     .max().unwrap();
        let max_col_runs = self.cols.iter()
                                    .map(|col| col.runs.len())
                                    .max().unwrap();

        let mut result = String::new();
        let grid = self.grid.borrow();

        for i in (0..max_col_runs).rev() {
            result.push_str(&self._fmt_header(i, prefix_len, subdivision, emit_color));
        }

        // top board line
        result.push_str(&Self::_fmt_line(
            &ralign("", prefix_len),
            "\u{2554}",
            "\u{2557}",
            "\u{2564}",
            subdivision,
            &(0..self.width()).map(|_| String::from("\u{2550}\u{2550}\u{2550}"))
                              .collect::<Vec<_>>(),
            emit_color,
        ));

        for y in 0..self.height() {
            // board content line
            result.push_str(&Self::_fmt_line(
                &ralign_joined_coloreds(&row_prefixes[y], prefix_len, emit_color),
                "\u{2551}",
                "\u{2551}",
                "\u{2502}",
                subdivision,
                &grid.squares[y].iter()
                                .map(|s| format!(" {:1} ", s))
                                .collect::<Vec<_>>(),
                emit_color,
            ));

            // horizontal subdivisor line
            if let Some(subdiv) = subdivision {
                if ((y+1) % subdiv == 0) && (y != self.height()-1) {
                    result.push_str(&Self::_fmt_line(
                        &ralign("", prefix_len),
                        "\u{255F}",
                        "\u{2562}",
                        "\u{253C}",
                        subdivision,
                        &(0..self.width()).map(|_| String::from("\u{2500}\u{2500}\u{2500}"))
                                          .collect::<Vec<_>>(),
                        emit_color,
                    ));
                }
            }
        }
        // bottom board line
        result.push_str(&Self::_fmt_line(
            &ralign("", prefix_len),
            "\u{255A}",
            "\u{255D}",
            "\u{2567}",
            subdivision,
            &(0..self.width()).map(|_| String::from("\u{2550}\u{2550}\u{2550}"))
                              .collect::<Vec<_>>(),
            emit_color,
        ));

        return result;
    }

    fn _fmt_line(prefix: &str,
                 left_delim: &str,
                 right_delim: &str,
                 columnwise_separator: &str,
                 subdivision: Option<usize>,
                 content_parts: &Vec<String>,
                 _emit_color: bool)
        -> String
    {
        let mut result = format!("{} {}", prefix, left_delim);
        for (idx, s) in content_parts.iter().enumerate() {
            result.push_str(s);
            if let Some(subdiv) = subdivision {
                if ((idx+1) % subdiv == 0) && (idx < content_parts.len()-1) {
                    result.push_str(columnwise_separator);
                }
            }
        }
        result.push_str(&format!("{}\n", right_delim));
        return result;
    }

    fn _fmt_header(&self, line_idx: usize,
                          prefix_len: usize,
                          subdivision: Option<usize>,
                          emit_color: bool)
        -> String
    {
        let mut content_parts = Vec::<String>::new();
        for col in &self.cols {
            let part: String;
            if line_idx < col.runs.len() {
                let colored = col.runs[col.runs.len()-1-line_idx].to_colored_string();
                part = format!(" {}", lalign_colored(&colored, 2, emit_color));
            } else {
                part = format!(" {:-2}", " ");
            }

            content_parts.push(part);
        }

        Self::_fmt_line(
            &ralign("", prefix_len),
            " ",
            " ",
            " ",
            subdivision,
            &content_parts,
            emit_color,
        )
    }
}
impl fmt::Display for Puzzle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let subdivision = Some(5);
        write!(f, "{}", self._fmt(subdivision, false))
    }
}

