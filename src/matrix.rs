use std::time::Duration;

use crate::{
    blocks::{Blocks, BLOCK_SIZE},
    piece::Piece,
};

use ggez::{
    graphics::{self, Color, DrawParam, Mesh, MeshBuilder},
    nalgebra::Point2,
    timer, Context, GameResult,
};
use rand_distr::{Distribution, Uniform};

pub const WIDTH: i32 = 10;
pub const HEIGHT: i32 = 20;
pub const VANISH: i32 = 20;

type Grid = [[usize; WIDTH as usize]; (HEIGHT + VANISH) as usize];

pub struct Matrix {
    grid: Grid,
    grid_mesh: Mesh,

    clearing: Option<(Vec<i32>, Duration)>,
}

impl Matrix {
    pub fn new(ctx: &mut Context) -> GameResult<Matrix> {
        let grid_mesh = &mut MeshBuilder::new();
        let grid_color = Color::new(0.2, 0.2, 0.2, 1.0);

        for x in 0..=WIDTH {
            let x = (x * BLOCK_SIZE) as f32;

            grid_mesh.line(
                &[
                    Point2::new(x, 0.0),
                    Point2::new(x, (BLOCK_SIZE * HEIGHT) as f32),
                ],
                1.0,
                grid_color,
            )?;
        }

        for y in 0..=HEIGHT {
            let y = (y * BLOCK_SIZE) as f32;

            grid_mesh.line(
                &[
                    Point2::new(0.0, y),
                    Point2::new((BLOCK_SIZE * WIDTH) as f32, y),
                ],
                1.0,
                grid_color,
            )?;
        }

        let grid_mesh = grid_mesh.build(ctx)?;

        Ok(Matrix {
            grid: [[0; WIDTH as usize]; (HEIGHT + VANISH) as usize],
            grid_mesh,
            clearing: None,
        })
    }

    pub fn clear(&mut self) {
        self.grid = [[0; WIDTH as usize]; (HEIGHT + VANISH) as usize];
    }

    pub fn collision(&self, piece: &Piece) -> bool {
        let grid = piece.get_grid();
        let x = piece.x + grid.offset_x;
        let y = piece.y + grid.offset_y;

        if x < 0 || x + grid.width > WIDTH || y < 0 || y + grid.height > HEIGHT + VANISH {
            return true;
        }

        for my in 0..grid.height {
            for mx in 0..grid.width {
                let c = grid.grid[(my + grid.offset_y) as usize][(mx + grid.offset_x) as usize];
                if c != 0 && self.grid[(y + my) as usize][(x + mx) as usize] != 0 {
                    return true;
                }
            }
        }

        false
    }

    pub fn lock(&mut self, piece: &Piece) -> bool {
        let grid = piece.get_grid();
        let x = piece.x + grid.offset_x;
        let y = piece.y + grid.offset_y;

        if y + grid.height <= VANISH {
            return false;
        }

        for my in 0..grid.height {
            for mx in 0..grid.width {
                let c = grid.grid[(my + grid.offset_y) as usize][(mx + grid.offset_x) as usize];
                if c != 0 {
                    self.grid[(y + my) as usize][(x + mx) as usize] = c;
                }
            }
        }

        self.clear_full_rows();
        true
    }

    pub fn blocked(&self) -> bool {
        self.clearing.is_some()
    }

    pub fn update(&mut self, ctx: &mut Context) {
        let mut clear = false;
        if let Some((_, duration)) = &mut self.clearing {
            *duration += timer::delta(ctx);

            if *duration > Duration::from_millis(500) {
                clear = true;
            }
        }

        if clear {
            let rows = self
                .clearing
                .as_ref()
                .unwrap_or(&(vec![], Duration::new(0, 0)))
                .0
                .clone();

            self.collapse_rows(&rows);
            self.clearing = None;
        }
    }

    pub fn draw(
        &mut self,
        ctx: &mut Context,
        position: Point2<f32>,
        blocks: &mut Blocks,
    ) -> GameResult {
        graphics::draw(ctx, &self.grid_mesh, DrawParam::new().dest(position))?;

        blocks.clear();

        for y in 0..=HEIGHT {
            for x in 0..WIDTH {
                let block = self.grid[(VANISH + y - 1) as usize][x as usize];
                if block == 0 {
                    continue;
                }

                let dest = Point2::new(
                    position[0] + (x * BLOCK_SIZE) as f32,
                    position[1] + ((y - 1) * BLOCK_SIZE) as f32,
                );

                blocks.add(block, dest);
            }
        }

        blocks.draw(ctx)?;

        Ok(())
    }

    fn clear_full_rows(&mut self) {
        let rows = self.get_full_rows();
        self.erase_rows(&rows);

        self.clearing = Some((rows, Duration::new(0, 0)));
    }

    fn get_full_rows(&self) -> Vec<i32> {
        let mut rows = vec![];

        for y in 0..HEIGHT + VANISH {
            let mut full = true;

            for x in 0..WIDTH {
                if self.grid[y as usize][x as usize] == 0 {
                    full = false;
                    break;
                }
            }

            if full {
                rows.push(y);
            }
        }

        rows
    }

    fn erase_rows(&mut self, rows: &[i32]) {
        for &y in rows {
            for x in 0..WIDTH {
                self.grid[y as usize][x as usize] = 0;
            }
        }
    }

    fn collapse_rows(&mut self, rows: &[i32]) {
        for &row in rows {
            for y in (1..=row).rev() {
                for x in 0..WIDTH {
                    self.grid[y as usize][x as usize] = self.grid[y as usize - 1][x as usize];
                }
            }
        }
    }

    pub fn debug_tower(&mut self) {
        let mut bricks: Vec<(usize, usize)> = vec![
            (39, 0),
            (39, 1),
            (38, 0),
            (37, 0),
            (37, 1),
            (36, 0),
            (36, 1),
            (35, 0),
            (34, 0),
            (34, 1),
            (33, 0),
            (33, 1),
            (32, 0),
            (31, 0),
            (31, 1),
            (30, 0),
            (30, 1),
            (29, 0),
            (28, 0),
            (28, 1),
            (26, 2),
            (25, 2),
        ];

        for y in 0..14 {
            bricks.push((39 - y, 3));
        }

        for y in 0..12 {
            for x in 4..10 {
                bricks.push((39 - y, x));
            }
        }

        self.clear();
        let mut rng = rand::thread_rng();
        let uniform = Uniform::new(1, 8);

        for (y, x) in bricks {
            self.grid[y][x] = uniform.sample(&mut rng);
        }
    }
}
