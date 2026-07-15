use crate::renderer::{DrawPass, Texture};
use crate::types::Color;

use std::sync::Arc;

pub struct Sprite {
    pub texture: Arc<Texture>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub src_x: f32,
    pub src_y: f32,
    pub src_w: f32,
    pub src_h: f32,
    pub tint: Color,
}

impl Sprite {
    pub fn new(texture: Arc<Texture>) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: texture.width as f32,
            height: texture.height as f32,
            src_x: 0.0,
            src_y: 0.0,
            src_w: texture.width as f32,
            src_h: texture.height as f32,
            texture,
            tint: Color::new(1.0, 1.0, 1.0, 1.0),
        }
    }

    pub fn draw(&self, pass: &mut DrawPass) {
        pass.draw_sprite(
            self.x, self.y, self.width, self.height,
            self.src_x, self.src_y, self.src_w, self.src_h,
            self.texture.width as f32, self.texture.height as f32,
            self.tint,
            &self.texture.bind_group,
        );
    }
}
