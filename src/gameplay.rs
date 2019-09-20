use std::{collections::VecDeque, time::Duration};

use ggez::{
    graphics::{Color, Font, Scale},
    nalgebra::{Point2, Vector2},
    timer, Context, GameResult,
};

use crate::{
    action::Action,
    bag::Bag,
    blocks::Blocks,
    global::Global,
    holder::Holder,
    matrix::{self, Locked, Matrix},
    piece::Piece,
    replay::ReplayData,
    score::Score,
    utils,
};

pub struct Gameplay {
    interactive: bool,
    action_duration: Duration,

    actions: VecDeque<Action>,
    replay: ReplayData,

    matrix: Matrix,
    bag: Bag,
    piece: Piece,
    holder: Holder,
    score: Score,

    game_over: bool,
    still: Duration,
    fall_interval: Duration,

    font: Font,
    blocks: Blocks,

    explosion: bool,
}

impl Gameplay {
    pub fn new(
        ctx: &mut Context,
        g: &mut Global,
        interactive: bool,
        seed: &[u8; 32],
    ) -> GameResult<Gameplay> {
        let actions = VecDeque::new();
        let replay = ReplayData::new(seed);

        let matrix = Matrix::new();

        let mut bag = Bag::new(seed);
        let piece = Piece::new(bag.pop());
        let holder = Holder::default();
        let score = Score::default();

        let font = Font::new(ctx, utils::path(ctx, "fonts/game.ttf"))?;

        let blocks = Blocks::new(g.settings.tileset(ctx, &g.settings_state)?);

        Ok(Gameplay {
            interactive,
            action_duration: Duration::new(0, 0),
            actions,
            replay,
            matrix,
            bag,
            piece,
            holder,
            score,
            game_over: false,
            still: Duration::new(0, 0),
            fall_interval: Duration::from_secs(1),
            font,
            blocks,
            explosion: false,
        })
    }

    fn reset_fall(&mut self) {
        if self.still > self.fall_interval {
            self.still -= self.fall_interval
        } else {
            self.still = Duration::new(0, 0);
        }
    }

    pub fn explode(&mut self) {
        self.explosion = true;
    }

    pub fn action(&mut self, action: Action) {
        self.actions.push_back(action);
        self.replay.add(action, self.action_duration);
        self.action_duration = Duration::new(0, 0);
    }

    pub fn actions(&mut self, actions: &[Action]) {
        for &action in actions {
            self.action(action);
        }
    }

    pub fn explosion(&mut self) -> bool {
        let result = self.explosion;
        self.explosion = false;
        result
    }

    pub fn replay_data(&self) -> &ReplayData {
        &self.replay
    }

    pub fn score(&self) -> i32 {
        self.score.score()
    }

    pub fn game_over(&self) -> bool {
        self.game_over
    }

    pub fn update(&mut self, ctx: &mut Context, g: &Global) -> GameResult<()> {
        if g.imgui_state.game_over {
            self.action(Action::GameOver);
        }

        if g.imgui_state.debug_t_spin_tower {
            self.matrix.debug_tower();
        }

        if g.settings_state.skin_switched {
            self.blocks = Blocks::new(g.settings.tileset(ctx, &g.settings_state)?);
        }

        self.matrix.update(ctx);
        if self.game_over || self.matrix.blocked() || g.imgui_state.paused {
            return Ok(());
        }

        self.action_duration += timer::delta(ctx);

        while let Some(action) = self.actions.pop_front() {
            // self.actions_history.push(action);

            match action {
                Action::MoveRight => {
                    if self.piece.shift(1, 0, &self.matrix)
                        && self.piece.touching_floor(&self.matrix)
                    {
                        self.reset_fall();
                    }
                }
                Action::MoveLeft => {
                    if self.piece.shift(-1, 0, &self.matrix)
                        && self.piece.touching_floor(&self.matrix)
                    {
                        self.reset_fall();
                    }
                }
                Action::MoveDown => {
                    if self.piece.shift(0, 1, &self.matrix) {
                        self.reset_fall();
                    }
                }
                Action::RotateClockwise => {
                    if self.piece.rotate(true, &self.matrix)
                        && self.piece.touching_floor(&self.matrix)
                    {
                        self.reset_fall();
                    }
                }
                Action::RotateCounterClockwise => {
                    if self.piece.rotate(false, &self.matrix)
                        && self.piece.touching_floor(&self.matrix)
                    {
                        self.reset_fall();
                    }
                }
                Action::SoftDrop => {
                    let rows = self.piece.fall(&self.matrix);
                    if rows > 0 {
                        self.reset_fall();
                        self.score.soft_drop(rows);
                    }
                }
                Action::HardDrop => {
                    let rows = self.piece.fall(&self.matrix);
                    self.score.hard_drop(rows);

                    if self.interactive {
                        self.action(Action::LockPiece);
                    }
                }
                Action::HoldPiece => {
                    if let Some(shape) = self.holder.hold(self.piece.shape(), &mut self.bag) {
                        self.piece = Piece::new(shape);
                    }
                }
                Action::FallPiece => {
                    if !self.piece.shift(0, 1, &self.matrix) && self.interactive {
                        self.action(Action::LockPiece);
                    }
                }
                Action::LockPiece => {
                    match self.matrix.lock(&self.piece) {
                        Locked::Collision => {
                            if self.interactive {
                                self.action(Action::GameOver);
                            }
                        }
                        Locked::Success(rows) => {
                            if rows > 0 {
                                self.explode();
                                self.score.lock(rows, self.piece.t_spin());
                            } else {
                                self.score.reset_combo();
                            }

                            self.piece = Piece::new(self.bag.pop());
                            if self.matrix.collision(&self.piece) && self.interactive {
                                self.action(Action::GameOver);
                            } else {
                                self.reset_fall();
                                self.holder.unlock();
                            }

                            if self.matrix.blocked() {
                                break;
                            }
                        }
                    };
                }
                Action::GameOver => {
                    self.game_over = true;
                    self.matrix.game_over();
                    self.explode();
                }
            };
        }

        if self.interactive {
            self.still += timer::delta(ctx);

            if self.still >= self.fall_interval {
                self.still -= self.fall_interval;

                self.action(Action::FallPiece);
            }
        }

        Ok(())
    }

    pub fn draw(&mut self, ctx: &mut Context, g: &Global, position: Point2<f32>) -> GameResult<()> {
        let block_size = g.settings.block_size;

        let ui_block_size = ((block_size * 3) as f32 / 4.0) as i32;
        let ui_color = Color::new(0.8, 0.9, 1.0, 0.8);
        let ui_font = self.font;
        let ui_scale = Scale::uniform(ui_block_size as f32);

        self.holder.draw(
            ctx,
            position + Vector2::new(-3.25 * ui_block_size as f32, 0.0),
            &mut self.blocks,
            ui_block_size,
            ui_color,
            ui_font,
        )?;

        self.bag.draw(
            ctx,
            position + Vector2::new(((matrix::WIDTH + 1) * block_size) as f32, 0.0),
            &mut self.blocks,
            ui_block_size,
            ui_color,
            ui_font,
        )?;

        self.score.draw(
            ctx,
            position
                + Vector2::new(
                    block_size as f32 * (matrix::WIDTH + 1) as f32,
                    block_size as f32 * (matrix::HEIGHT - 2) as f32,
                ),
            ui_color,
            ui_font,
            ui_scale,
        )?;

        self.matrix
            .draw(ctx, position, &mut self.blocks, block_size)?;

        if !self.game_over {
            self.piece
                .draw(ctx, position, &mut self.blocks, block_size, 1.0)?;

            if g.settings.ghost_piece > 0.0 {
                let mut ghost = self.piece.clone();
                if ghost.fall(&self.matrix) >= ghost.grid().height {
                    ghost.draw(
                        ctx,
                        position,
                        &mut self.blocks,
                        block_size,
                        g.settings.ghost_piece,
                    )?;
                }
            }
        }

        Ok(())
    }
}
