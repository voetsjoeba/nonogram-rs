// vim: set ai et ts=4 sw=4 sts=4:
use std::fmt;
use std::convert::TryFrom;
use std::ops::{Index, IndexMut};
use yaml_rust::Yaml;

use super::square::{Square, SquareStatus};
use super::util::{ralign, Direction, Direction::Horizontal, Direction::Vertical};
use super::run::Run;
use super::row::Row;
use super::field::Field;

#[derive(Debug)]
pub struct Puzzle {
    rows:    Vec<Row>,
    cols:    Vec<Row>,
    squares: Vec<Vec<Square>>,
}

impl Puzzle {

    pub fn from_yaml(doc: &Yaml) -> Puzzle {
        let rows = Self::_parse_row(&doc["rows"], Horizontal);
        let cols = Self::_parse_row(&doc["cols"], Vertical);

        let width = cols.len();
        let height = rows.len();

        let mut squares = Vec::<Vec::<Square>>::with_capacity(height);
        for y in 0..height {
            let row: Vec<Square> = (0..width).map(|x| Square::new(x, y)).collect();
            squares.push(row);
        }

        Puzzle {
            rows: rows,
            cols: cols,
            squares: squares,
        }
    }
    fn _parse_row(input: &Yaml, direction: Direction) -> Vec<Row> {
		let list: &Vec<Yaml> = input.as_vec().unwrap();
        list.iter()
            .enumerate()
		    .map(|(i, yaml_val)| Row::new(direction, u32::try_from(i).unwrap(),
                                          Self::_parse_row_runs(yaml_val, direction)))
			.collect()
    }
    fn _parse_row_runs(input: &Yaml, direction: Direction) -> Vec<Run> {
        match input {
            Yaml::String(_)  => { input.as_str().unwrap()
                                       .split_whitespace()
                                       .map(|int| Run::new(direction, int.trim().parse().unwrap()))
                                       .collect()
                                },
            Yaml::Integer(_) => { vec![ Run::new(direction,
			                                     u32::try_from(input.as_i64().unwrap()).unwrap()) ]
							    }
            _ => panic!("Unexpected data type: {:?}", input),
        }
    }

    pub fn width(&self) -> usize {
        self.squares[0].len()
    }
    pub fn height(&self) -> usize {
        self.squares.len()
    }

}

impl Index<usize> for Puzzle {
    type Output = Vec<Square>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.squares[index]
    }
}
impl IndexMut<usize> for Puzzle {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.squares[index]
    }
}

impl fmt::Display for Puzzle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
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
        fn _fmt_header(puzzle: &Puzzle, line_idx: usize, prefix_len: usize, f: &mut fmt::Formatter) {
            _fmt_line(f,
                      &ralign("", prefix_len),
                      " ",
                      " ",
                      " ",
                      &puzzle.cols.iter()
                                  .map( |col| format!(" {:-2}", if line_idx < col.runs.len() { col.runs[col.runs.len()-1-line_idx].length.to_string() } else { String::from("") }) )
                                      .collect::<Vec<_>>()
            )
        }

        let row_prefixes: Vec<String> = self.rows.iter()
                                            .map(|row| row.runs.iter().map(|run| run.length.to_string())
                                                                      .collect::<Vec<_>>()
                                                                      .join(" "))
                                            .collect();
        let prefix_len = row_prefixes.iter().map(|x| x.len()).max().unwrap();
        let max_col_runs = self.cols.iter().map(|col| col.runs.len()).max().unwrap();


        for i in (0..max_col_runs).rev() {
            _fmt_header(self, i, prefix_len, f);
        }

        // top board line
        _fmt_line(f,
                  &ralign("", prefix_len),
                  "\u{2554}",
                  "\u{2557}",
                  "\u{2564}",
                  &(0..self.width()).map(|_| String::from("\u{2550}\u{2550}\u{2550}"))
                                    .collect::<Vec<_>>()
        );
        for y in 0..self.height() {
            // board content line
            _fmt_line(f,
                      &ralign(&row_prefixes[y], prefix_len),
                      "\u{2551}",
                      "\u{2551}",
                      "\u{2502}",
                      &self.squares[y].iter()
                                      .map(|s| format!(" {:1} ", s))
                                      .collect::<Vec<_>>()
            );

            // horizontal board separator line
            if ((y+1) % 5 == 0) && (y != self.height()-1) {
                _fmt_line(f,
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
        _fmt_line(f,
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

