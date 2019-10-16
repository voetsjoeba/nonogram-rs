// vim: set ai et ts=4 sts=4 sw=4:
extern crate yaml_rust;
use std::vec::Vec;
use std::convert::TryFrom;
use std::fmt;
use yaml_rust::{YamlLoader, Yaml};

#[derive(Debug)]
enum SquareStatus {
    FilledIn,
    CrossedOut,
    Unknown,
}

#[derive(Debug)]
struct Square {
    row: usize,
    col: usize,
    status: SquareStatus,
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self.status {
            SquareStatus::CrossedOut => "\u{26AC}", // medium circle, not filled in
            SquareStatus::FilledIn   => "\u{25A0}", // filled in black square
            SquareStatus::Unknown    => " ",
        })
    }
}

#[derive(Debug)]
struct Puzzle {
    row_runs: Vec<Vec<u32>>,
    col_runs: Vec<Vec<u32>>,
    squares:  Vec<Vec<Square>>,
}

fn ralign(s: &str, width: usize) -> String {
    if s.len() >= width {
        return String::from(s);
    }
    format!("{}{}", " ".repeat(width-s.len()), s)
}

impl fmt::Display for Puzzle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn _fmt_line(f: &mut fmt::Formatter,
                     prefix: &str,
                     left_delim: &str,
                     right_delim: &str,
                     columnwise_separator: &str,
                     content_parts: &Vec<String>)
        {
            write!(f, "{prefix} {left_delim}", prefix=prefix, left_delim=left_delim).expect("");
            for (x, s) in content_parts.iter().enumerate() {
                write!(f, "{}", s);
                if ((x+1) % 5 == 0) && (x < content_parts.len()-1) {
                    write!(f, "{}", columnwise_separator);
                }
            }
            write!(f, "{right_delim}\n", right_delim=right_delim).expect("");
        }
        fn _fmt_header(puzzle: &Puzzle, line_idx: usize, prefix_len: usize, f: &mut fmt::Formatter) {
            _fmt_line(f,
                      &ralign("", prefix_len),
                      " ",
                      " ",
                      "   ",
                      &puzzle.col_runs.iter()
                                      .map( |x| format!(" {:-2}", if line_idx < x.len() { x[x.len()-1-line_idx].to_string() } else { String::from("") }) )
                                      .collect::<Vec<_>>()
            )
        }

        let row_prefixes: Vec<String> = self.row_runs.iter()
                                            .map(|x| x.iter().map(|y| y.to_string())
                                                             .collect::<Vec<_>>()
                                                             .join(" "))
                                            .collect();
        let prefix_len = row_prefixes.iter().map(|x| x.len()).max().unwrap();
        let max_col_runs = self.col_runs.iter().map(|x| x.len()).max().unwrap();


        for i in (0..max_col_runs).rev() {
            _fmt_header(self, i, prefix_len, f);
        }

        // top board line
        _fmt_line(f,
                  &ralign("", prefix_len),
                  "\u{2554}",
                  "\u{2557}",
                  "\u{2550}\u{2564}\u{2550}",
                  &(0..self.width()).map(|_| String::from("\u{2550}\u{2550}\u{2550}"))
                                    .collect::<Vec<_>>()
        );
        for y in 0..self.height() {
            // board content line
            _fmt_line(f,
                      &ralign(&row_prefixes[y], prefix_len),
                      "\u{2551}",
                      "\u{2551}",
                      " \u{2502} ",
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
                          "\u{2500}\u{253C}\u{2500}",
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
                  "\u{2550}\u{2567}\u{2550}",
                  &(0..self.width()).map(|_| String::from("\u{2550}\u{2550}\u{2550}"))
                                    .collect::<Vec<_>>()
        );

        return Ok(())
    }
}

impl Puzzle {

    fn from_yaml(doc: &Yaml) -> Puzzle {
        let row_runs = Self::_parse_runs(&doc["rows"]);
        let col_runs = Self::_parse_runs(&doc["cols"]);

        let width = col_runs.len();
        let height = row_runs.len();

        let mut squares = Vec::<Vec::<Square>>::with_capacity(height);
        for y in 0..height {
            let row : Vec<Square> = (0..width).map(|x| Square { col: x, row: y, status: SquareStatus::CrossedOut }).collect();
            squares.push(row);
        }

        squares[0][2].status = SquareStatus::FilledIn;
        squares[1][2].status = SquareStatus::FilledIn;
        squares[2][2].status = SquareStatus::FilledIn;

        Puzzle {
            row_runs: row_runs,
            col_runs: col_runs,
            squares: squares,
        }
    }
    fn _parse_run_spec(input: &Yaml) -> Vec<u32> {
        match input {
            Yaml::String(_)  => { input.as_str().unwrap()
                                       .split_whitespace()
                                       .map(|x| x.trim().parse().unwrap())
                                       .collect()
                                },
            Yaml::Integer(_) => { vec![ u32::try_from(input.as_i64().unwrap()).unwrap() ] }
                           _ => panic!("Unexpected data type: {:?}", input),
        }
    }
    fn _parse_runs(input: &Yaml) -> Vec<Vec<u32>> {
        let mut result = Vec::<Vec<u32>>::new();
        for value in input.as_vec().unwrap().iter() {
            let ints = Self::_parse_run_spec(value);
            result.push(ints);
        }
        return result;
    }

    fn width(&self) -> usize {
        self.squares[0].len()
    }
    fn height(&self) -> usize {
        self.squares.len()
    }

}


fn main() {
    let s = "
rows:
    - 1
    - 3
    - 1
    - 1 1 1
    - 3
    - 1
    - 3
    - 1
    - 1 1 1
    - 3
cols:
    - 1
    - 1 2 3
    - 5
    - 4 5 6
    - 1
    - 1
    - 1 2 3
    - 5
    - 4 5 6
    - 1
";
    // note: column numbers are listed top to bottom
    let docs: Vec<Yaml> = YamlLoader::load_from_str(s).unwrap();
    let doc: &Yaml = &docs[0];

    let puzzle = Puzzle::from_yaml(doc);
    println!("{}", puzzle);
}
