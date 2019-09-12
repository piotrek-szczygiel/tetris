use std::{ffi::OsStr, time::Duration};

use crate::{
    bag::Bag,
    blocks::Blocks,
    global::Global,
    holder::Holder,
    input::{Action, Input},
    matrix::{self, Matrix},
    particles::ParticleAnimation,
    piece::Piece,
    utils,
};

use ggez::{
    audio::{self, SoundSource},
    filesystem,
    graphics::{self, Color, DrawParam, Font, Image, Scale, Text, TextFragment},
    input::keyboard::KeyCode,
    nalgebra::{Point2, Vector2},
    timer, Context, GameResult,
};
use imgui::ImString;

pub struct Game {
    input: Input,

    matrix: Matrix,
    bag: Bag,
    piece: Piece,
    holder: Holder,

    game_over: bool,
    still: Duration,
    fall_interval: Duration,

    font: Font,
    blocks: Blocks,
    particle_animation: ParticleAnimation,
    background: Image,
    music: audio::Source,
}

impl Game {
    pub fn new(ctx: &mut Context, g: &mut Global) -> GameResult<Game> {
        let repeat = Some((150, 50));
        let mut input = Input::new();
        input
            .bind(KeyCode::Right, Action::MoveRight, repeat)
            .bind(KeyCode::Left, Action::MoveLeft, repeat)
            .bind(KeyCode::Down, Action::MoveDown, repeat)
            .bind(KeyCode::Up, Action::RotateClockwise, None)
            .bind(KeyCode::X, Action::RotateClockwise, None)
            .bind(KeyCode::Z, Action::RotateCounterClockwise, None)
            .bind(KeyCode::Space, Action::HardFall, None)
            .bind(KeyCode::LShift, Action::SoftFall, None)
            .bind(KeyCode::C, Action::HoldPiece, None)
            .exclude(KeyCode::Right, KeyCode::Left)
            .exclude(KeyCode::Left, KeyCode::Right);

        let matrix = Matrix::new();
        let mut bag = Bag::new();
        let piece = Piece::new(bag.pop());
        let holder = Holder::new();
        let font = Font::new(ctx, utils::path(ctx, "font.ttf"))?;

        let rect = graphics::screen_coordinates(ctx);
        let particle_animation = ParticleAnimation::new(130, 200.0, 80.0, rect.w, rect.h);

        let background = Image::new(ctx, utils::path(ctx, "background.jpg"))?;

        let mut music = audio::Source::new(ctx, utils::path(ctx, "main_theme.ogg"))?;
        music.set_repeat(true);
        music.set_volume(0.2);
        music.play()?;

        g.settings_state.skins = filesystem::read_dir(ctx, utils::path(ctx, "blocks"))?
            .filter(|p| p.extension().unwrap_or_else(|| OsStr::new("")) == "png")
            .collect();
        g.settings_state.skins_imstr = g
            .settings_state
            .skins
            .iter()
            .map(|s| ImString::from(String::from(s.file_name().unwrap().to_str().unwrap())))
            .collect();

        let blocks = Blocks::new(g.settings.tileset(ctx, &g.settings_state)?);

        Ok(Game {
            input,
            matrix,
            bag,
            piece,
            holder,
            game_over: false,
            still: Duration::new(0, 0),
            fall_interval: Duration::from_secs(1),
            font,
            blocks,
            particle_animation,
            background,
            music,
        })
    }

    fn lock_piece(&mut self) {
        if !self.matrix.lock(&self.piece) {
            self.game_over = true;
        } else {
            self.piece = Piece::new(self.bag.pop());
            if self.matrix.collision(&self.piece) {
                self.game_over = true;
            } else {
                self.reset_fall();
                self.holder.unlock();
            }
        }
    }

    fn reset_fall(&mut self) {
        if self.still > self.fall_interval {
            self.still -= self.fall_interval
        } else {
            self.still = Duration::new(0, 0);
        }
    }

    pub fn update(&mut self, ctx: &mut Context, g: &Global) -> GameResult<()> {
        if g.settings.animated_background {
            self.particle_animation.update(ctx)?;
        }

        if g.imgui_state.debug_t_spin_tower {
            self.matrix.debug_tower();
        }

        if g.settings_state.skin_switched {
            self.blocks = Blocks::new(g.settings.tileset(ctx, &g.settings_state)?);
        }

        if (self.music.volume() - g.settings.music_volume).abs() > 0.01 {
            self.music.set_volume(g.settings.music_volume);
        }

        self.matrix.update(ctx);
        if self.game_over || self.matrix.blocked() || g.imgui_state.paused {
            return Ok(());
        }

        self.input.update(ctx);

        while let Some(action) = self.input.action() {
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
                Action::SoftFall => {
                    let rows = self.piece.fall(&self.matrix);
                    if rows > 0 {
                        self.reset_fall();
                    }
                }
                Action::HardFall => {
                    self.piece.fall(&self.matrix);
                    self.lock_piece();
                }
                Action::HoldPiece => {
                    if let Some(shape) = self.holder.hold(self.piece.shape(), &mut self.bag) {
                        self.piece = Piece::new(shape);
                    }
                }
            };
        }

        self.still += timer::delta(ctx);

        if self.still >= self.fall_interval {
            self.still -= self.fall_interval;

            if !self.piece.shift(0, 1, &self.matrix) {
                self.lock_piece();
            }
        }

        Ok(())
    }

    pub fn draw(&mut self, ctx: &mut Context, g: &Global) -> GameResult<()> {
        let coords = graphics::screen_coordinates(ctx);
        let ratio = coords.w / coords.h;

        graphics::draw(
            ctx,
            &self.background,
            graphics::DrawParam::new().scale(Vector2::new(
                if ratio > (21.0 / 9.0) {
                    ratio / (21.0 / 9.0)
                } else {
                    1.0
                },
                1.0,
            )),
        )?;

        let block_size = g.settings.block_size;

        self.particle_animation.draw(ctx)?;

        let position = Point2::new(
            (coords.w - (matrix::WIDTH * block_size) as f32) / 2.0,
            (coords.h - (matrix::HEIGHT * block_size) as f32) / 2.0,
        );

        let ui_block_size = ((block_size * 3) as f32 / 4.0) as i32;

        let hold_text = Text::new(TextFragment {
            text: "hold".to_string(),
            color: Some(Color::new(0.8, 0.9, 1.0, 0.8)),
            font: Some(self.font),
            scale: Some(Scale::uniform(block_size as f32)),
        });

        graphics::draw(
            ctx,
            &hold_text,
            DrawParam::new().dest(position - Vector2::new(ui_block_size as f32 * 4.5, 0.0)),
        )?;

        self.holder.draw(
            ctx,
            position + Vector2::new(-3.25 * ui_block_size as f32, ui_block_size as f32 * 2.0),
            &mut self.blocks,
            ui_block_size,
        )?;

        let next_text = Text::new(TextFragment {
            text: "next".to_string(),
            color: Some(Color::new(0.8, 0.9, 1.0, 0.8)),
            font: Some(self.font),
            scale: Some(Scale::uniform(block_size as f32)),
        });

        graphics::draw(
            ctx,
            &next_text,
            DrawParam::new().dest(
                position
                    + Vector2::new(
                        ((matrix::WIDTH) * block_size) as f32 + ui_block_size as f32 * 2.1,
                        0.0,
                    ),
            ),
        )?;

        self.bag.draw(
            ctx,
            position
                + Vector2::new(
                    ((matrix::WIDTH + 1) * block_size) as f32,
                    ui_block_size as f32 * 2.0,
                ),
            &mut self.blocks,
            ui_block_size,
        )?;

        self.matrix
            .draw(ctx, position, &mut self.blocks, block_size)?;

        self.piece
            .draw(ctx, position, &mut self.blocks, block_size, 1.0)?;

        if g.settings.ghost_piece {
            let mut ghost = self.piece.clone();
            if ghost.fall(&self.matrix) >= ghost.grid().height {
                ghost.draw(ctx, position, &mut self.blocks, block_size, 0.1)?;
            }
        }

        Ok(())
    }
}
