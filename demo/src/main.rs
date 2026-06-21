use libdqg::input::InputState;
use libdqg::renderer::DrawPass;
use libdqg::scene::{Scene, SceneTransition};
use libdqg::game::{GameBuilder};
use libdqg::types::{Color, KeyCode};
use rand::RngExt;

struct MyOtherScene {
    rng: rand::rngs::ThreadRng,
}

impl MyOtherScene {
    fn new() -> Self {
        Self {
            rng: rand::rng(),
        }
    }
}

impl Scene for MyOtherScene {
    fn update(&mut self, _delta_time: f32, input_state: &InputState) -> SceneTransition {
        if input_state.is_key_pressed(KeyCode::Space) {
            return SceneTransition::Next;
        }
        // Update code here
        SceneTransition::None
    }

    fn render(&mut self, _pass: &mut DrawPass) {
        // Drawing commands go here
    }
}

struct MyScene {}

impl Scene for MyScene {
    fn update(&mut self, _delta_time: f32, input_state: &InputState) -> SceneTransition {
        if input_state.is_key_pressed(KeyCode::Space) {
            return SceneTransition::Previous;
        }
        SceneTransition::None
    }

    fn render(&mut self, _pass: &mut DrawPass) {
        // Drawing commands go here
    }
}

fn main() {
    let mut game = GameBuilder::new(Box::new(MyScene{}))
        .add_scene(Box::new(MyOtherScene::new()))
        .title("My Game".into())
        .size(800, 600)
        .build();
    game.run();
}
