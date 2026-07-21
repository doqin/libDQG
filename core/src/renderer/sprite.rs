use crate::renderer::{DrawPass, Texture};
use crate::transform::Transformable;
use crate::types::Color;

use std::sync::Arc;

pub struct Sprite {
    pub texture: Arc<Texture>,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub width: f32,
    pub height: f32,
    pub src_x: f32,
    pub src_y: f32,
    pub src_w: f32,
    pub src_h: f32,
    pub tint: Color,
    /// Model matrix applied to the quad in the sprite's local space, where the origin is the
    /// quad's anchor corner and the quad spans `(0, 0)..(width, height)`. The transformed quad
    /// is then positioned at `(x, y, z)`, so translation here is relative to that position.
    ///
    /// Defaults to [`glam::Mat4::IDENTITY`]. Prefer building it up through the
    /// [`Transformable`] methods, which rotate and scale around the sprite's center.
    pub transform: glam::Mat4,
}

impl Sprite {
    pub fn new(texture: Arc<Texture>) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            width: texture.width as f32,
            height: texture.height as f32,
            src_x: 0.0,
            src_y: 0.0,
            src_w: texture.width as f32,
            src_h: texture.height as f32,
            texture,
            tint: Color::new(1.0, 1.0, 1.0, 1.0),
            transform: glam::Mat4::IDENTITY,
        }
    }

    /// Draws the sprite in screen space (pixel coordinates), ignoring the camera. Use for UI/HUD elements.
    pub fn draw(&self, pass: &mut DrawPass) {
        pass.draw_ui_sprite(
            self.x, self.y, self.width, self.height,
            self.src_x, self.src_y, self.src_w, self.src_h,
            self.texture.width as f32, self.texture.height as f32,
            self.tint,
            self.transform,
            &self.texture.bind_group,
        );
    }

    /// Draws the sprite as a quad in world space, transformed by the camera.
    pub fn draw_world(&self, pass: &mut DrawPass) {
        pass.draw_world_sprite(
            self.x, self.y, self.z, self.width, self.height,
            self.src_x, self.src_y, self.src_w, self.src_h,
            self.texture.width as f32, self.texture.height as f32,
            self.tint,
            self.transform,
            &self.texture.bind_group,
        );
    }
}

/// Rotations and scales pivot around the middle of the sprite's quad, so a sprite spins in
/// place rather than swinging around its anchor corner.
///
/// ```no_run
/// # use libdqg::{renderer::Sprite, Transformable};
/// # fn demo(sprite: &mut Sprite, angle: f32) {
/// sprite.reset_transform().rotate(angle).scale_uniform(2.0);
/// # }
/// ```
impl Transformable for Sprite {
    fn transform(&self) -> glam::Mat4 {
        self.transform
    }

    fn transform_mut(&mut self) -> &mut glam::Mat4 {
        &mut self.transform
    }

    fn pivot(&self) -> glam::Vec3 {
        glam::Vec3::new(self.width * 0.5, self.height * 0.5, 0.0)
    }
}
