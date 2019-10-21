// vim: set ai et ts=4 sw=4 sts=4:
mod solver;

use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;
use std::convert::TryFrom;
use yaml_rust::Yaml;

use super::grid::{Grid, Square, SquareStatus};
use super::util::{ralign, Direction, Direction::*};
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
}

impl Puzzle {
    // helper functions for Puzzle::fmt
    fn _fmt_line(f: &mut fmt::Formatter,
                 prefix: &str,
                 left_delim: &str,
                 right_delim: &str,
                 columnwise_separator: &str,
                 content_parts: &Vec<String>)
    {
        write!(f, "{prefix} {left_delim}", prefix=prefix, left_delim=left_delim).expect("");
        for (idx, s) in content_parts.iter().enumerate() {
            write!(f, "{}", s).expect("");
            if ((idx+1) % 5 == 0) && (idx < content_parts.len()-1) {
                write!(f, "{}", columnwise_separator).expect("");
            }
        }
        write!(f, "{right_delim}\n", right_delim=right_delim).expect("");
    }

    fn _fmt_header(&self,
                   line_idx: usize,
                   prefix_len: usize,
                   f: &mut fmt::Formatter)
    {
        let mut content_parts = Vec::<String>::new();
        for col in &self.cols {
            let part: String;

            if line_idx < col.runs.len() {
                part = col.runs[col.runs.len()-1-line_idx].length.to_string();
            } else {
                part = String::from("");
            }

            content_parts.push(format!(" {:-2}", part));
        }

        Self::_fmt_line(f,
                        &ralign("", prefix_len),
                        " ",
                        " ",
                        " ",
                        &content_parts
        )
    }
}
impl fmt::Display for Puzzle {
    fn fmt(&self,
           f: &mut fmt::Formatter) -> fmt::Result
    {
        let row_prefixes = self.rows.iter()
                                    .map(|row| row.runs.iter()
                                                       .map(|run| run.to_string())
                                                       .collect::<Vec<_>>()
                                                       .join(" "))
                                    .collect::<Vec<_>>();

        let prefix_len = row_prefixes.iter()
                                     .map(|x| x.len())
                                     .max()
                                     .unwrap();
        let max_col_runs = self.cols.iter()
                                    .map(|col| col.runs.len())
                                    .max()
                                    .unwrap();
        let grid = self.grid.borrow();

        for i in (0..max_col_runs).rev() {
            self._fmt_header(i, prefix_len, f);
        }

        // top board line
        Self::_fmt_line(f,
                        &ralign("", prefix_len),
                        "\u{2554}",
                        "\u{2557}",
                        "\u{2564}",
                        &(0..self.width()).map(|_| String::from("\u{2550}\u{2550}\u{2550}"))
                                          .collect::<Vec<_>>()
        );

        for y in 0..self.height() {
            // board content line
            Self::_fmt_line(f,
                            &ralign(&row_prefixes[y], prefix_len),
                            "\u{2551}",
                            "\u{2551}",
                            "\u{2502}",
                            &grid.squares[y].iter()
                                            .map(|s| format!(" {:1} ", s))
                                            .collect::<Vec<_>>()
            );

            // horizontal board separator line
            if ((y+1) % 5 == 0) && (y != self.height()-1) {
                Self::_fmt_line(f,
                                &ralign("", prefix_len),
                                "\u{255F}",
                                "\u{2562}",
                                "\u{253C}",
                                &(0..self.width()).map(|_| String::from("\u{2500}\u{2500}\u{2500}"))
                                                  .collect::<Vec<_>>()
                );
            }
        }
        // bottom board line
        Self::_fmt_line(f,
                        &ralign("", prefix_len),
                        "\u{255A}",
                        "\u{255D}",
                        "\u{2567}",
                        &(0..self.width()).map(|_| String::from("\u{2550}\u{2550}\u{2550}"))
                                          .collect::<Vec<_>>()
        );

        return Ok(())
    }
}

