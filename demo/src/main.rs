use libdqg::scene::Scene;
use libdqg::game::run_game;

struct MyGame {}

impl Scene for MyGame {
    fn update(&mut self) -> libdqg::scene::SceneTransition {
        // Update code here
        libdqg::scene::SceneTransition::None
    }

    fn render(&mut self, renderer: &mut libdqg::renderer::Renderer) {
        renderer.clear();
    }
}

fn main() {
    run_game(Box::new(MyGame {}));
}
