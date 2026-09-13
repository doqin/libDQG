mod editor_scene;
mod fly_camera;
mod picking;
mod ui;

use editor_scene::EditorScene;
use libdqg::game::GameBuilder;
use libdqg::types::Color;

fn main() {
    let mut game = GameBuilder::new(Box::new(EditorScene::new()))
        .title("libDQG Editor".into())
        .size(1280, 800)
        .integrated_titlebar(true)
        .clear_color(Color::new(0.05, 0.05, 0.07, 1.0))
        .resizable(true)
        .build();
    game.run();
}
