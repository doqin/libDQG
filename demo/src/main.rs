use libdqg::input::InputState;
use libdqg::renderer::DrawPass;
use libdqg::scene::{Scene, SceneTransition};
use libdqg::game::GameBuilder;
use libdqg::types::{Color, KeyCode};

struct MyOtherScene;

impl Scene for MyOtherScene {
    fn update(&mut self, _delta_time: f32, input_state: &InputState) -> SceneTransition {
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

struct MyScene;

impl Scene for MyScene {
    fn update(&mut self, _delta_time: f32, input_state: &InputState) -> SceneTransition {
        if input_state.is_key_pressed(KeyCode::Space) {
            return SceneTransition::Previous;
        }
        SceneTransition::None
    }

    fn render(&mut self, pass: &mut DrawPass) {
        pass.draw_rect(100.0, 100.0, 250.0, 180.0, 0.0, Color::new(0.0, 0.8, 0.0, 1.0));
        pass.draw_ellipse(500.0, 250.0, 90.0, 60.0, 32, 3.0, Color::new(1.0, 0.8, 0.0, 1.0));
        pass.draw_line(200.0, 400.0, 600.0, 500.0, 5.0, Color::new(0.0, 1.0, 1.0, 1.0));
    }
}

fn main() {
    let mut game = GameBuilder::new(Box::new(MyScene))
        .add_scene(Box::new(MyOtherScene))
        .title("My Game".into())
        .size(800, 600)
        .build();
    game.run();
}
