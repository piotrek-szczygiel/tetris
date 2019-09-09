use crate::{
    bag::Bag,
    blocks::Blocks,
    shape::{Shape, ShapeType},
};

use ggez::{nalgebra::Point2, Context, GameResult};

pub struct Holder {
    shape: Option<Shape>,
    locked: bool,
}

impl Holder {
    pub fn new() -> Holder {
        Holder {
            shape: None,
            locked: false,
        }
    }

    pub fn hold(&mut self, shape_type: ShapeType, bag: &mut Bag) -> Option<ShapeType> {
        if self.locked {
            return None;
        }

        self.locked = true;

        let mut swap = Some(Shape::new(shape_type));
        std::mem::swap(&mut self.shape, &mut swap);

        swap.map_or(Some(bag.pop()), |s| Some(s.shape_type))
    }

    pub fn unlock(&mut self) {
        self.locked = false;
    }

    pub fn draw(
        &self,
        ctx: &mut Context,
        position: Point2<f32>,
        blocks: &mut Blocks,
        block_size: i32,
    ) -> GameResult {
        if let Some(shape) = &self.shape {
            shape.draw(ctx, 0, position, blocks, block_size, 1.0)?;
        }

        Ok(())
    }
}
