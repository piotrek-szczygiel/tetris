use ggez::{
    graphics::{self, Color, DrawParam, Font, Scale, Text, TextFragment},
    nalgebra::Point2,
    Context, GameResult,
};

#[derive(Default)]
pub struct Score {
    score: i32,
    last_clear: i32,
    combo: Option<i32>,
}

impl Score {
    pub fn score(&self) -> i32 {
        self.score
    }

    pub fn soft_drop(&mut self, rows: i32) {
        self.score += rows;
    }

    pub fn hard_drop(&mut self, rows: i32) {
        self.score += rows * 2;
    }

    pub fn reset_combo(&mut self) {
        self.combo = None;
    }

    pub fn lock(&mut self, rows: i32, t_spin: bool) {
        let mut score = 0;
        let mut _garbage = 0;

        // For back-to-back
        let last_hard = self.last_clear >= 800;

        match (rows, t_spin) {
            (1, false) => {
                score = 100;
                _garbage = 0;
            }
            (1, true) => {
                score = 800;
                _garbage = 2;
            }
            (2, false) => {
                score = 300;
                _garbage = 1;
            }
            (2, true) => {
                score = 1200;
                _garbage = 4;
            }
            (3, false) => {
                score = 500;
                _garbage = 3;
            }
            (3, true) => {
                score = 1600;
                _garbage = 6;
            }
            (4, false) => {
                score = 800;
                _garbage = 4;
            }
            _ => (),
        }

        if last_hard {
            _garbage += 1;

            if score >= 800 {
                score += score / 2;
            }
        }

        if let Some(mut combo) = self.combo {
            combo += 1;
            score += 50 * combo;
        } else {
            self.combo = Some(0);
        }

        self.last_clear = score;
        self.score += score;
    }

    pub fn draw(
        &self,
        ctx: &mut Context,
        position: Point2<f32>,
        color: Color,
        font: Font,
        scale: Scale,
    ) -> GameResult {
        let text = Text::new(TextFragment {
            text: format!("score: {}", self.score),
            color: Some(color),
            font: Some(font),
            scale: Some(scale),
        });

        graphics::draw(ctx, &text, DrawParam::new().dest(position))?;

        Ok(())
    }
}
