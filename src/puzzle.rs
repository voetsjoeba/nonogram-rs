// vim: set ai et ts=4 sw=4 sts=4:
use std::fmt;
use std::io;
use std::rc::Rc;
use std::cell::RefCell;
use std::convert::TryFrom;
use std::collections::{VecDeque, HashSet};
use yaml_rust::Yaml;
use ansi_term::ANSIString;

use super::grid::{Grid, Square, SquareStatus, Change, Changes, Error, HasGridLocation};
use super::util::{ralign, lalign_colored, ralign_joined_coloreds, Direction, Direction::*, is_a_tty};
use super::row::{Row, Run, Field};

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

    pub fn is_completed(&self) -> bool {
        self.rows.iter().all(|r| r.is_completed()) &&
            self.cols.iter().all(|c| c.is_completed())
    }
    pub fn solve(&mut self) -> Result<(), Error> {
        // keep a queue of rows to be looked at, and run the individual solvers on each
        // of them in sequence until there are none left in the queue. whenever a change
        // is made to a square in the grid, those rows are added back into the queue
        // for evaluation on the next run. completed runs are removed from the queue.
        println!("starting state:");
        println!("\n{}", self);

        let mut queue = VecDeque::<(Direction, usize)>::new();
        queue.extend(self.rows.iter().map(|r| (r.direction, r.index)));
        queue.extend(self.cols.iter().map(|c| (c.direction, c.index)));

        let mut cur_iteration = 0usize;
        let max_iterations = 100000usize;
        while let Some((d,i)) = queue.pop_front()
        {
            cur_iteration += 1;
            if cur_iteration >= max_iterations {
                panic!("max iterations exceeded, aborting");
            }

            let mut changes = Vec::<Change>::new();
            {
                let row = match d {
                    Horizontal => &mut self.cols[i],
                    Vertical   => &mut self.rows[i],
                };
                
                row.recalculate_fields();
                row.update_run_bounds();
                changes.extend(row.fill_overlap()?);
                changes.extend(row.infer_run_assignments()?);

                changes.extend(row.check_completed_runs()?);
                changes.extend(row.check_completed()?);
            }

            if changes.len() > 0
            {
                println!("finished solvers on {} row {}; changes in this iteration:", d, i);
                for change in &changes {
                    println!("  {}", change);
                }

                // we made changes to one or more squares in the grid; for each square that was affected,
                // add the horizontal and vertical rows that cross it back into the queue for re-evaluation
                // (if they aren't already completed)
                for change in &changes {
                    let h_row = &self.rows[change.get_row()];
                    let v_row = &self.cols[change.get_col()];

                    if !h_row.is_completed() {
                        let value = (h_row.direction, h_row.index);
                        if !queue.contains(&value) { queue.push_back(value); }
                    }
                    if !v_row.is_completed() {
                        let value = (v_row.direction, v_row.index);
                        if !queue.contains(&value) { queue.push_back(value); }
                    }
                }


                println!("\n{}", self);
                println!("--------------------------------------");
            }

        }

        if self.is_completed() {
            println!("puzzle solved!");
        } else {
            println!("puzzle partially solved, out of actions.");
        }
        Ok(())
    }
}

impl Puzzle {
    // helper functions for Puzzle::fmt
    fn _fmt(&self, subdivision: Option<usize>)
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
            result.push_str(&self._fmt_header(i, prefix_len, subdivision));
        }

        // top board line
        result.push_str(&Self::_fmt_line(
            &ralign("", prefix_len),
            "\u{2554}",
            "\u{2557}",
            "\u{2564}",
            subdivision,
            &(0..self.width()).map(|_| String::from("\u{2550}\u{2550}\u{2550}"))
                              .collect::<Vec<_>>()
        ));

        for y in 0..self.height() {
            // board content line
            result.push_str(&Self::_fmt_line(
                &ralign_joined_coloreds(&row_prefixes[y], prefix_len),
                "\u{2551}",
                "\u{2551}",
                "\u{2502}",
                subdivision,
                &grid.squares[y].iter()
                                .map(|s| format!(" {:1} ", s))
                                .collect::<Vec<_>>()
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
                                          .collect::<Vec<_>>()
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
                              .collect::<Vec<_>>()
        ));

        return result;
    }

    fn _fmt_line(prefix: &str,
                 left_delim: &str,
                 right_delim: &str,
                 columnwise_separator: &str,
                 subdivision: Option<usize>,
                 content_parts: &Vec<String>)
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
                          subdivision: Option<usize>)
        -> String
    {
        let mut content_parts = Vec::<String>::new();
        for col in &self.cols {
            let part: String;
            if line_idx < col.runs.len() {
                let colored = col.runs[col.runs.len()-1-line_idx].to_colored_string();
                part = format!(" {}", lalign_colored(&colored, 2));
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
            &content_parts
        )
    }
}
impl fmt::Display for Puzzle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        //let subdivision = None;
        let subdivision = Some(5);
        write!(f, "{}", self._fmt(subdivision))
    }
}

