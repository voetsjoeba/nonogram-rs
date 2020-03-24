// vim: set ai et ts=4 sts=4 sw=4:
use super::puzzle::{Puzzle, Solver};
use super::grid::SquareStatus;
use super::row::{Row, DirectionalSequence};
use super::util::{Direction::*};
use super::Args;

use std::convert::TryFrom;
use std::fmt;
use piston::window::WindowSettings;
use piston::event_loop::{Events, EventLoop, EventSettings};
use piston::input::{RenderEvent, GenericEvent, Button, Key};
use glutin_window::GlutinWindow;
use graphics::{Context, Graphics, clear};
use graphics::{Rectangle, Line, Transformed, Image, Text};
use graphics::types::Color;
use graphics::character::CharacterCache;
use opengl_graphics::{OpenGL, GlGraphics, Filter, GlyphCache, TextureSettings};

struct PuzzleController {
    //pub puzzle: Puzzle,
    pub solver: Solver,
    pub cursor_pos: [f64;2],
}
impl PuzzleController {
    pub fn new(puzzle: Puzzle) -> Self {
        PuzzleController {
            solver: Solver::new(puzzle),
            cursor_pos: [-1.0,-1.0]
        }
    }
    pub fn event<E: GenericEvent>(&mut self, e: &E) {
        if let Some(pos) = e.mouse_cursor_args() {
            self.cursor_pos = pos;
        }
        if let Some(Button::Keyboard(key)) = e.press_args() {
            match key {
                Key::S => {
                    // single-step the solver
                    if let Some(iteration_result) = self.solver.next() {
                        match iteration_result {
                            Ok((d,i,changes)) => { }
                            Err(_) => { }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
struct PuzzleViewSettings {
    pub position: [f64; 2],
    pub subdivision_size: Option<usize>, // visual subdivision size (optional)
    pub square_size: f64, // width and height of each square

    pub unknown_sq_fill_color: Color,
    pub unknown_sq_fill_color_hl: Color,
    pub filled_sq_fill_color: Color,
    pub filled_sq_fill_color_hl: Color,
    pub crossedout_sq_line_color: Color,
    pub crossedout_sq_line_thickness: f64,

    pub line_color: Color,
    pub square_line_thickness: f64, // line width for individual squares
    pub subdivision_line_thickness: f64, // line width for subdivision separators
    pub outline_line_thickness: f64, // line width for the grid border

    pub run_text_font_size: u32,
    pub run_text_color_hl: Color,
    pub run_text_color_complete: Color,
    pub run_text_color_incomplete: Color,

    pub info_text_font_size: u32,
    pub info_text_color: Color,
    pub info_text_line_height: f64,

}
impl PuzzleViewSettings {
    pub fn new(subdivision_size: Option<usize>) -> Self {
        Self {
            position: [20.0; 2],
            subdivision_size,
            square_size: 20.0,

            unknown_sq_fill_color: [0.7, 0.7, 0.7, 1.0],
            unknown_sq_fill_color_hl: [0.8, 0.8, 0.8, 1.0],
            filled_sq_fill_color: [99.0/255.0, 128.0/255.0, 1.0, 1.0],
            filled_sq_fill_color_hl: [138.0/255.0, 182.0/255.0, 1.0, 1.0], // highlight
            crossedout_sq_line_color: [0.8, 0.8, 0.8, 1.0],
            crossedout_sq_line_thickness: 0.75,

            line_color: [0.0, 0.0, 0.0, 1.0],
            square_line_thickness: 1.0,
            subdivision_line_thickness: 2.0,
            outline_line_thickness: 3.0,

            run_text_font_size: 18,
            //run_text_color_hl: [236.0/255.0, 153.0/255.0, 23.0/255.0, 1.0],
            run_text_color_hl: [1.0, 0.0, 0.0, 1.0],
            run_text_color_complete: [0.7, 0.7, 0.7, 1.0],
            run_text_color_incomplete: [0.0, 0.0, 0.0, 1.0],

            info_text_font_size: 16,
            info_text_color: [0.0, 0.0, 0.0, 1.0],
            info_text_line_height: 20.0,
        }
    }
}
struct PuzzleView {
    pub settings: PuzzleViewSettings,
}
impl PuzzleView {
    pub fn new(settings: PuzzleViewSettings) -> Self {
        Self { settings }
    }
    pub fn mouse_pos_to_square(&self, puzzle: &Puzzle, pos: [f64; 2])
        -> Option<[usize;2]>
    {
        // given a mouse position (in absolute coordinates), returns the (x,y) indices of the corresponding
        // square (if any) in the given puzzle.
        
        // the square grid starts at self.settings.position + the width and height of the run areas
        // TODO: code duplication with draw()
        let square_size: f64 = self.settings.square_size;
        let num_h_runs: usize = puzzle.rows.iter().map(|row| row.runs.len()).max().unwrap();
        let num_v_runs: usize = puzzle.cols.iter().map(|col| col.runs.len()).max().unwrap();
        let runarea_drawwidth: f64  = (num_h_runs as f64) * square_size; // width of the runs block to the left of the grid
        let runarea_drawheight: f64 = (num_v_runs as f64) * square_size; // width of the runs block to the top of the grid

        let grid_xoffset = self.settings.position[0] + runarea_drawwidth;
        let grid_yoffset = self.settings.position[1] + runarea_drawheight;
        let mouse_x_relative = pos[0] - grid_xoffset; // relative to the top left corner of the drawn grid
        let mouse_y_relative = pos[1] - grid_yoffset;

        let square_x = (mouse_x_relative / square_size).floor() as isize;
        let square_y = (mouse_y_relative / square_size).floor() as isize;
        if square_x >= 0 && square_x < (puzzle.width() as isize) &&
           square_y >= 0 && square_y < (puzzle.height() as isize)
        {
            Some([square_x as usize, square_y as usize])
        } else {
            None
        }
        
    }
    pub fn draw_h_runs<G: Graphics, C>(&self, row: &Row,
                                              draw_width: f64,
                                              highlighted_idx: Option<usize>,
                                              c: &Context,
                                              glyphs: &mut C,
                                              g: &mut G)
        where C: CharacterCache<Texture = G::Texture>
    {
        let square_size = self.settings.square_size;
        for (n,run) in row.runs.iter().rev().enumerate() {
            let mut text_color = match run.is_completed() {
                true  => self.settings.run_text_color_complete,
                false => self.settings.run_text_color_incomplete,
            };
            if let Some(h_idx) = highlighted_idx {
                if run.index == h_idx { text_color = self.settings.run_text_color_hl; }
            }
            let text_style = Text::new_color(text_color, self.settings.run_text_font_size);

            let mut x = draw_width - square_size/4.0 - ((n+1) as f64) * square_size; // subtract a little extra for visual margin
            let y = ((row.index + 1) as f64) * square_size; // text y position is on bottom left, not top left
            if run.length < 10 { x += square_size/4.0; } // move single-char numbers over a bit
            let c = c.trans(x, y-(square_size/6.0)); // move text up a little bit for visual
            text_style.draw(&run.length.to_string(), glyphs, &c.draw_state, c.transform, g)
                      .ok().unwrap();
        }
    }
    pub fn draw_v_runs<G: Graphics, C>(&self, row: &Row,
                                              draw_height: f64,
                                              highlighted_idx: Option<usize>,
                                              c: &Context,
                                              glyphs: &mut C,
                                              g: &mut G)
        where C: CharacterCache<Texture = G::Texture>
    {
        let square_size = self.settings.square_size;
        for (i,run) in row.runs.iter().rev().enumerate() {
            let mut text_color = match run.is_completed() {
                true  => self.settings.run_text_color_complete,
                false => self.settings.run_text_color_incomplete,
            };
            if let Some(h_idx) = highlighted_idx {
                if run.index == h_idx { text_color = self.settings.run_text_color_hl; }
            }
            let text_style = Text::new_color(text_color, self.settings.run_text_font_size);

            let mut x = (row.index as f64) * square_size;
            let y = draw_height - square_size/4.0 - (i as f64) * square_size;
            if run.length < 10 { x += square_size/4.0; } // move single-char numbers over a bit
            let c = c.trans(x, y);
            text_style.draw(&run.length.to_string(), glyphs, &c.draw_state, c.transform, g)
                      .ok().unwrap();
        }
    }
    pub fn draw_square<G: Graphics>(&self, x: usize,
                                           y: usize,
                                           is_highlighted: bool,
                                           controller: &PuzzleController,
                                           c: &Context,
                                           g: &mut G)
    {
        // note: we're in a translated context, so we can draw our square starting at (0,0) in the top left
        let square_size = self.settings.square_size;
        let square_rect = [0.0, 0.0, square_size, square_size];

        let square = controller.solver.puzzle.get_square(x, y);
        match square.get_status() {
            SquareStatus::FilledIn   => {
                let fill_style = Rectangle::new(match is_highlighted {
                    true  => self.settings.filled_sq_fill_color_hl,
                    false => self.settings.filled_sq_fill_color,
                });
                fill_style.draw(square_rect, &c.draw_state, c.transform, g);
            }
            SquareStatus::CrossedOut => {
                let margin = square_size/5.0;
                let line_style = Line::new(self.settings.crossedout_sq_line_color,
                                           self.settings.crossedout_sq_line_thickness);

                line_style.draw([margin, margin, square_size-margin, square_size-margin], &c.draw_state, c.transform, g);
                line_style.draw([square_size-margin, margin, margin, square_size-margin], &c.draw_state, c.transform, g);
            }
            SquareStatus::Unknown    => {
                let fill_style = Rectangle::new(match is_highlighted {
                    true  => self.settings.unknown_sq_fill_color_hl,
                    false => self.settings.unknown_sq_fill_color,
                });
                fill_style.draw(square_rect, &c.draw_state, c.transform, g);
            }
        }

        // if the square has known vertical or horizontal runs, draw a small indicator line to signify this
        if let Some(_) = square.get_run_index(Horizontal) {
            let line_style = Line::new([0.0, 0.0, 0.0, 1.0], 0.5);
            line_style.draw([0.0, square_size/2.0, square_size/2.0 * 0.8, square_size/2.0], &c.draw_state, c.transform, g);
        }
        if let Some(_) = square.get_run_index(Vertical) {
            let line_style = Line::new([0.0, 0.0, 0.0, 1.0], 0.5);
            line_style.draw([square_size/2.0, 0.0, square_size/2.0, square_size/2.0 * 0.8], &c.draw_state, c.transform, g);
        }
    }
    pub fn draw<G: Graphics, C>(&self, controller: &PuzzleController,
                                       c: &Context,
                                       glyphs: &mut C,
                                       g: &mut G)
        where C: CharacterCache<Texture = G::Texture>
    {
		let settings = &self.settings;
        let line_color = settings.line_color;

        // note:
        // rectangle [x, y, width, height] with border:
        //   * filled part of rectangle drawn from (x,y) to (x+w, y+h)
        //   * border is drawn on top, centered on edge lines:
        //        first pixel goes inside, next pixel goes outside, next one inside again, ...
        //        if radius is of half-pixel size, the border is drawn on the inside at one side of the rectangle
        //        of the rectangle
        //
        // e.g. [x, y, 5, 5] with border radius 1:
        //   +---+-------------------+---+
        //   |                           |
        //   +   A---+---+---+---+---+   +    A = (x, y)
        //   |   |                   |   |    -> fill not exposed here, obscured by border
        //   |   +   +---+---+---+   +   |
        //   |   |   |///|///|///|   |   |    -> area marked with /// is where filled is exposed
        //   |   +   +///+///+///+   +   |   
        //   |   |   |///|///|///|   |   |
        //   |   +   +///+///+///+   +   |
        //   |   |   |///|///|///|   |   |
        //   |   +   +---+---+---+   +   |
        //   |   |                   |   |
        //   +   +---+---+---+---+---B   +    B = (x+w, y+h)
        //   |                           |
        //   +---+-------------------+---+
        //
        // e.g. in a [x, y, 10, 10] with border radius 0.5, the exposed fill area will be 9x9.

        // note:
        // line [x1, x2, y1, y2],
        //  => radius is HALF the line thickness!
        let c = c.trans(settings.position[0], settings.position[1]);

        let subdivision_size = settings.subdivision_size.unwrap_or(0usize);
        let square_size = settings.square_size;
        let puzzle = &controller.solver.puzzle;

        // rectangles are specified by: [x, y, w, h]
        // lines are specified by: [x1, y1, x2, y2]
        let num_h_runs = puzzle.rows.iter().map(|row| row.runs.len()).max().unwrap();
        let num_v_runs = puzzle.cols.iter().map(|col| col.runs.len()).max().unwrap();
        let runarea_drawwidth = (num_h_runs as f64) * square_size; // width of the runs block to the left of the grid
        let runarea_drawheight = (num_v_runs as f64) * square_size; // width of the runs block to the top of the grid

        let grid_xoffset = runarea_drawwidth;
        let grid_yoffset = runarea_drawheight;
        let grid_drawwidth  = (puzzle.width() as f64) * square_size;
        let grid_drawheight = (puzzle.height() as f64) * square_size;

        let highlighted_sq_pos = self.mouse_pos_to_square(&controller.solver.puzzle, controller.cursor_pos);

        // draw squares
        for y in 0..puzzle.height() {
            for x in 0..puzzle.width() {
                let is_highlighted = highlighted_sq_pos.map(|[hx, hy]| hx == x && hy == y).unwrap_or(false);
                let c = c.trans(grid_xoffset + (x as f64)*square_size,
                                grid_yoffset + (y as f64)*square_size);
                self.draw_square(x, y, is_highlighted, controller, &c, g);
            }
        }

        // draw run numbers
        for row_idx in 0..puzzle.height() {
            let row = &puzzle.rows[row_idx];
            let mut highlighted_run_idx: Option<usize> = None;
            if let Some([hx,hy]) = highlighted_sq_pos {
                if row_idx == hy {
                    highlighted_run_idx = puzzle.get_square(hx, hy).get_run_index(row.direction);
                }
            }
            self.draw_h_runs(row, runarea_drawwidth, highlighted_run_idx, &c.trans(0.0, grid_yoffset), glyphs, g);
        }
        for col_idx in 0..puzzle.width() {
            let col = &puzzle.cols[col_idx];
            let mut highlighted_run_idx: Option<usize> = None;
            if let Some([hx,hy]) = highlighted_sq_pos {
                if col_idx == hx {
                    highlighted_run_idx = puzzle.get_square(hx, hy).get_run_index(col.direction);
                }
            }
            self.draw_v_runs(col, runarea_drawheight, highlighted_run_idx, &c.trans(grid_xoffset, 0.0), glyphs, g);
        }

        // draw grid
        {
            let square_line_style = Line::new(line_color, settings.square_line_thickness/2.0); // line radius = HALF of line thickness!
            let subdivision_line_style = Line::new(line_color, settings.subdivision_line_thickness/2.0);
            let grid_outline_style = Line::new(line_color, settings.outline_line_thickness/2.0);

            for i in 0..puzzle.height()+1 { // +1 for extra line to cleanly close the grid
                let y = runarea_drawheight + (i as f64) * square_size;
                let line_coords = [0.0, y, runarea_drawwidth + grid_drawwidth, y];

                let style = match i {
                    a if a == 0 || a == puzzle.height()                     => &grid_outline_style,
                    a if subdivision_size > 0 && a % subdivision_size == 0  => &subdivision_line_style,
                    _                                                       => &square_line_style,
                };
                style.draw(line_coords, &c.draw_state, c.transform, g);
            }
            for i in 0..puzzle.width()+1 { // +1 for extra line to cleanly close the grid
                let x = runarea_drawwidth + (i as f64) * square_size;
                let line_coords = [x, 0.0, x, runarea_drawheight + grid_drawheight];

                let style = match i {
                    a if a == 0 || a == puzzle.width()                      => &grid_outline_style,
                    a if subdivision_size > 0 && a % subdivision_size == 0  => &subdivision_line_style,
                    _                                                       => &square_line_style,
                };
                style.draw(line_coords, &c.draw_state, c.transform, g);
            }
        }

        // draw some progress and state information
        {
            let c = c.trans(grid_xoffset + grid_drawwidth, 0.0);
            let c = c.trans(square_size, 0.0); // some extra spacing
            let text_style = Text::new_color([0.0, 0.0, 0.0, 1.0], settings.info_text_font_size);

            let num_squares_total = puzzle.height() * puzzle.width();
            let num_squares_known = puzzle.rows.iter().fold(0, |acc, row| acc + (0..row.length).filter(|&pos| row.get_square(pos).get_status() != SquareStatus::Unknown)
                                                                                               .count());
            let state_text = format!(
r"Completion: {}/{}
Iterations: {}

Press S to single-step the solver.", num_squares_known, num_squares_total,
                                     controller.solver.iterations);
            for (i, line) in state_text.split("\n").enumerate() {
                let c = c.trans(0.0, (i as f64) * settings.info_text_line_height);
                text_style.draw(line, glyphs, &c.draw_state, c.transform, g).ok().unwrap();
            }
        }

    }
}

pub fn ui_main(puzzle: Puzzle, args: &Args)
{
    let opengl_version = OpenGL::V3_2;
    let settings = WindowSettings::new("Nonogram", [1200, 800])
                                   .graphics_api(opengl_version)
                                   .exit_on_esc(true);
    let mut window: GlutinWindow = settings.build().expect("Could not create window");

    let mut events = Events::new(EventSettings::new());
    let mut gl = GlGraphics::new(opengl_version);

    let mut puzzle_controller = PuzzleController::new(puzzle);
    let puzzle_view_settings = PuzzleViewSettings::new(args.visual_groups);
    let puzzle_view = PuzzleView::new(puzzle_view_settings);

    let texture_settings = TextureSettings::new().filter(Filter::Nearest);
    let mut glyphs = GlyphCache::new("FiraSans-Regular.ttf", (), texture_settings)
        .expect("Could not load font");

    while let Some(e) = events.next(&mut window) {
        puzzle_controller.event(&e);
        if let Some(ev_args) = e.render_args() {
            gl.draw(ev_args.viewport(), |c, g| {
                clear([1.0;4], g);
                puzzle_view.draw(&puzzle_controller, &c, &mut glyphs, g);
            });
        }
    }
}
