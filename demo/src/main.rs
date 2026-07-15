use std::sync::Arc;

use libdqg::input::InputState;
use libdqg::renderer::{DrawPass, Sprite, Texture};
use libdqg::scene::{Scene, SceneTransition};
use libdqg::game::GameBuilder;
use libdqg::types::{Color, KeyCode};

struct Pos {
    x: f32,
    y: f32,
}

struct MyOtherScene;

impl Scene for MyOtherScene {
    fn update(&mut self, _delta_time: f32, input_state: &InputState, _renderer: Option<&libdqg::renderer::Renderer>) -> SceneTransition {
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
    rect_pos: Pos,
    sample_sprite: Option<Sprite>,
}

impl MyScene {
    fn new(rect_pos: Pos) -> Self {
        Self { rect_pos, sample_sprite: None }
    }
}

static SMUG_TEXTURE: &[u8] = include_bytes!("../asset/textures/smug.png");

impl Scene for MyScene {
    fn update(
        &mut self, 
        delta_time: f32, 
        input_state: &InputState, 
        renderer: Option<&libdqg::renderer::Renderer>
    ) -> SceneTransition {
        // === LOADING ===
        if let Some(renderer) = renderer {
            if self.sample_sprite.is_none() {
                let texture = Arc::new(Texture::from_bytes(renderer, SMUG_TEXTURE)
                    .expect("Failed to create texture from bytes"));
                let mut sprite = Sprite::new(texture);
                sprite.width = 128.0;
                sprite.height = 128.0;
                self.sample_sprite = Some(sprite);
            }
        }
        // === LOGIC ===
        if input_state.is_key_pressed(KeyCode::Space) {
            return SceneTransition::Previous;
        }
        if input_state.is_key_pressed(KeyCode::Escape) {
            return SceneTransition::Quit;
        }
        let mut dir = Pos { x: 0.0, y: 0.0 };
        let velocity = 200.0;

        if input_state.is_key_held(KeyCode::ArrowUp) {
            dir.y -= 1.0;
        }
        if input_state.is_key_held(KeyCode::ArrowDown) {
            dir.y += 1.0;
        }
        if input_state.is_key_held(KeyCode::ArrowLeft) {
            dir.x -= 1.0;
        }
        if input_state.is_key_held(KeyCode::ArrowRight) {
            dir.x += 1.0;
        }
        if let Some(sprite) = &mut self.sample_sprite {
            sprite.x += dir.x * velocity * delta_time;
            sprite.y += dir.y * velocity * delta_time;
        }
        SceneTransition::None
    }

    fn render(&mut self, pass: &mut DrawPass) {
        pass.draw_rect(self.rect_pos.x, self.rect_pos.y, 250.0, 180.0, 0.0, Color::new(0.0, 0.8, 0.0, 1.0));
        pass.draw_ellipse(500.0, 250.0, 90.0, 60.0, 32, 3.0, Color::new(1.0, 0.8, 0.0, 1.0));
        pass.draw_line(200.0, 400.0, 600.0, 500.0, 5.0, Color::new(0.0, 1.0, 1.0, 1.0));
        
        if let Some(sprite) = &self.sample_sprite {
            sprite.draw(pass);
        }
    }
}

fn main() {
    let mut game = GameBuilder::new(Box::new(MyScene::new(Pos { x: 100.0, y: 100.0 })))
        .add_scene(Box::new(MyOtherScene))
        .title("My Game".into())
        .size(800, 600)
        .integrated_titlebar(true)
        .clear_color(Color::from_hex(0xffffff))
        .resizable(false)
        .build();
    game.run();
}
