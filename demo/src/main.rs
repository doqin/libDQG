mod camera_controller;

use std::sync::Arc;

use libdqg::Transformable;
use libdqg::input::InputState;
use libdqg::renderer::{DrawPass, Model, Sprite, Texture};
use libdqg::scene::{Scene, SceneTransition};
use libdqg::game::GameBuilder;
use libdqg::types::{Color, KeyCode};

use crate::camera_controller::CameraController;

struct MyOtherScene;

impl Scene for MyOtherScene {
    fn update(&mut self, _delta_time: f32, input_state: &InputState, _renderer: Option<&mut libdqg::renderer::Renderer>) -> SceneTransition {
        if input_state.is_key_pressed(KeyCode::Space) {
            return SceneTransition::Next;
        }
        SceneTransition::None
    }

    fn render(&mut self, pass: &mut DrawPass) {
        pass.draw_rect(50.0, 50.0, 200.0, 150.0, 0.0, Color::new(1.0, 0.5, 0.0, 1.0));
        pass.draw_ellipse(400.0, 300.0, 100.0, 70.0, 32, 0.0, Color::new(0.2, 0.6, 1.0, 1.0));
        pass.draw_line(100.0, 500.0, 700.0, 200.0, 3.0, Color::new(0.0, 1.0, 0.0, 1.0));
        pass.draw_rect(600.0, 50.0, 140.0, 100.0, 4.0, Color::new(1.0, 0.0, 0.5, 0.8));
    }
}

struct MyScene {
    sample_sprite: Option<Sprite>,
    floor_sprite: Option<Sprite>,
    sample_model: Option<Model>,
    camera_controller: CameraController,
}

impl MyScene {
    fn new() -> Self {
        Self {
            sample_sprite: None,
            floor_sprite: None,
            sample_model: None,
            camera_controller: CameraController::new(5.0),
        }
    }
}

impl Scene for MyScene {
    fn update(
        &mut self,
        delta_time: f32,
        input_state: &InputState,
        renderer: Option<&mut libdqg::renderer::Renderer>
    ) -> SceneTransition {
        // === LOADING ===
        if let Some(renderer) = renderer.as_deref() {
            if self.sample_sprite.is_none() {
                let texture = Arc::new(Texture::from_path(renderer, "./res/textures/smug.png")
                    .expect("Failed to create texture from path"));
                let mut sprite = Sprite::new(texture);
                // World-space sizes are in world units, not pixels. The camera
                // sits ~2 units from the origin, so a pixel-sized quad would
                // extend far behind the near plane and get clipped.
                sprite.width = 1.0;
                sprite.height = 1.0;
                sprite.x = -0.5;
                sprite.y = -0.5;
                self.sample_sprite = Some(sprite);
            }
            if self.floor_sprite.is_none() {
                let texture = Arc::new(Texture::from_path(renderer, "./res/textures/floor.png")
                    .expect("Failed to create texture from path"));
                let mut floor_sprite = Sprite::new(texture);
                floor_sprite.width = 10.0;
                floor_sprite.height = 10.0;
                floor_sprite.x = -5.0;
                floor_sprite.y = -5.0;
                floor_sprite.rotate_x(90.0_f32.to_radians());
                self.floor_sprite = Some(floor_sprite);
            }
            if self.sample_model.is_none() {
                let mut model = Model::load(renderer, "./res/models/cube.obj")
                    .expect("Failed to load model");
                model.translate3(glam::Vec3::new(1.5, 0.5, -1.5));
                self.sample_model = Some(model);
            }
        }
        // === LOGIC ===
        if input_state.is_key_pressed(KeyCode::Space) {
            return SceneTransition::Previous;
        }
        if input_state.is_key_pressed(KeyCode::Escape) {
            return SceneTransition::Quit;
        }
        let velocity = 5.0;
        let rotation_speed = 180_f32.to_radians();

        if let Some(sprite) = &mut self.sample_sprite {
            if input_state.is_key_held(KeyCode::KeyW) {
                sprite.translate3(glam::Vec3::new(0.0, 0.0, -1.0) * velocity * delta_time);
            }
            if input_state.is_key_held(KeyCode::KeyS) {
                sprite.translate3(glam::Vec3::new(0.0, 0.0,1.0) * velocity * delta_time);
            }
            if input_state.is_key_held(KeyCode::KeyA) {
                sprite.rotate_y(rotation_speed * delta_time);
            }
            if input_state.is_key_held(KeyCode::KeyD) {
                sprite.rotate_y(-rotation_speed * delta_time);
            }
        }
        if let Some(floor_sprite) = &mut self.floor_sprite {
            if input_state.is_key_held(KeyCode::KeyE) {
                floor_sprite.translate3(glam::Vec3::new(0.0, 0.0, -1.0) * velocity * delta_time);
            }
            if input_state.is_key_held(KeyCode::KeyQ) {
                floor_sprite.translate3(glam::Vec3::new(0.0, 0.0, 1.0) * velocity * delta_time);
            }
        }
        if let Some(model) = &mut self.sample_model {
            model.rotate_y(rotation_speed * 0.5 * delta_time);
        }

        for code in input_state.keys_held() {
            self.camera_controller.handle_key(code);
        }
        if let Some(renderer) = renderer {
            let camera = renderer.camera_mut();
            if let Some(sprite) = &self.sample_sprite {
                camera.target = sprite.transform().transform_point3(glam::Vec3::ZERO);
            }
            self.camera_controller.update_camera(delta_time, camera);
        }
        SceneTransition::None
    }

    fn render(&mut self, pass: &mut DrawPass) {
        // pass.draw_rect(self.rect_pos.x, self.rect_pos.y, 250.0, 180.0, 0.0, Color::new(0.0, 0.8, 0.0, 1.0));
        // pass.draw_ellipse(500.0, 250.0, 90.0, 60.0, 32, 3.0, Color::new(1.0, 0.8, 0.0, 1.0));
        // pass.draw_line(200.0, 400.0, 600.0, 500.0, 5.0, Color::new(0.0, 1.0, 1.0, 1.0));

        if let Some(sprite) = &self.sample_sprite {
            sprite.draw_world(pass);
        }
        if let Some(floor_sprite) = &self.floor_sprite {
            floor_sprite.draw_world(pass);
        }
        if let Some(model) = &self.sample_model {
            pass.draw_model(model);
        }
    }
}

fn main() {
    let mut game = GameBuilder::new(Box::new(MyScene::new()))
        .add_scene(Box::new(MyOtherScene))
        .title("My Game".into())
        .size(800, 600)
        .integrated_titlebar(true)
        // .clear_color(Color::from_hex(0x0))
        .resizable(false)
        .build();
    game.run();
}
