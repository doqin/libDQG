use libdqg::input::InputState;
use libdqg::renderer::Renderer;
use libdqg::scene::{Scene, SceneTransition};
use libdqg::game::{GameBuilder};
use libdqg::types::{Color, KeyCode};
use rand::RngExt;

struct MyScene {
    rng: rand::rngs::ThreadRng,
}

impl MyScene {
    fn new() -> Self {
        Self {
            rng: rand::rng(),
        }
    }
}

impl Scene for MyScene {
    fn update(&mut self, _delta_time: f32, input_state: &InputState) -> SceneTransition {
        if input_state.is_key_pressed(KeyCode::Space) {
            return SceneTransition::Next;
        }
        // Update code here
        SceneTransition::None
    }

    fn render(&mut self, renderer: &mut Renderer) {
        renderer.clear(Color::new(
            self.rng.random_range(0.0..1.0), 
            self.rng.random_range(0.0..1.0), 
            self.rng.random_range(0.0..1.0), 
            1.0)
        );
    }
}

struct MyOtherScene {}

impl Scene for MyOtherScene {
    fn update(&mut self, _delta_time: f32, input_state: &InputState) -> SceneTransition {
        if input_state.is_key_pressed(KeyCode::Space) {
            return SceneTransition::Previous;
        }
        SceneTransition::None
    }

    fn render(&mut self, renderer: &mut Renderer) {
        renderer.clear(Color::new(0.0, 0.0, 1.0, 1.0));
    }
}

fn main() {
    let mut game = GameBuilder::new(Box::new(MyScene::new()))
        .add_scene(Box::new(MyOtherScene {}))
        .title("My Game".into())
        .size(800, 600)
        .build();
    game.run();
}
