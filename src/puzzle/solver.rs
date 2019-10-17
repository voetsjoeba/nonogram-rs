// vim: set ai et ts=4 sts=4 sw=4:
use super::Puzzle;
use super::super::row::{Row, Field};
use super::super::util::{Direction, Direction::*};
use super::super::grid::SquareStatus::{CrossedOut};

impl Puzzle {
    pub fn solve(&mut self) {
        // 1. update field definitions on each row (i.e. contiguous runs of non-crossedout squares)
        for row in &mut self.rows {
            Self::_update_row_fields(row, Horizontal);
        }

        // 2. update min_start and max_start values of each run
    }
    fn _update_row_fields(row: &mut Row, direction: Direction) {
        row.fields.clear();

        let mut x: usize = 0;
        while x < row.length {
            // skip past crossedout squares
            while x < row.length && row.get_square(x).get_status() == CrossedOut {
                x += 1;
            }
            if x >= row.length { break; }

            // scan until first crossed-out again square (if any) or the end of the row
            // otherwise
            let field_start = x;
            while x < row.length && row.get_square(x).get_status() != CrossedOut {
                x += 1;
            }
            let field_len = x - field_start;
            let field: Field = row.make_field(field_start, field_len);
            row.fields.push(field);
        }
    }

}
