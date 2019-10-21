// vim: set ai et ts=4 sts=4 sw=4:
use super::Puzzle;

impl Puzzle {
    pub fn solve(&mut self) {
        // 1. update field definitions on each row (i.e. contiguous runs of non-crossedout squares)
        // 2. update min_start and max_start values of each run

        for row in &mut self.rows {
            row.recalculate_fields();
            row.update_run_bounds();
            row.fill_overlap();
            row.mark_completed_runs();
        }
        for col in &mut self.cols {
            col.recalculate_fields();
            col.update_run_bounds();
            col.fill_overlap();
            col.mark_completed_runs();
        }
        println!("yay done solving");

    }

}
