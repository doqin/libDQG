use libdqg::scene::Scene;
use libdqg::game::run_game;
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
    fn update(&mut self, input_state: &libdqg::input::InputState) -> libdqg::scene::SceneTransition {
        if input_state.is_key_pressed(libdqg::types::KeyCode::Space) {
            return libdqg::scene::SceneTransition::Replace(Box::new(MyOtherScene {}));
        }
        // Update code here
        libdqg::scene::SceneTransition::None
    }

    fn render(&mut self, renderer: &mut libdqg::renderer::Renderer) {
        renderer.clear(libdqg::types::Color::new(
            self.rng.random_range(0.0..1.0), 
            self.rng.random_range(0.0..1.0), 
            self.rng.random_range(0.0..1.0), 
            1.0)
        );
    }
}

struct MyOtherScene {}

impl Scene for MyOtherScene {
    fn update(&mut self, input_state: &libdqg::input::InputState) -> libdqg::scene::SceneTransition {
        if input_state.is_key_pressed(libdqg::types::KeyCode::Space) {
            return libdqg::scene::SceneTransition::Replace(Box::new(MyScene::new()));
        }
        libdqg::scene::SceneTransition::None
    }

    fn render(&mut self, renderer: &mut libdqg::renderer::Renderer) {
        renderer.clear(libdqg::types::Color::new(0.0, 0.0, 1.0, 1.0));
    }
}

fn main() {
    run_game(Box::new(MyScene::new()));
}
