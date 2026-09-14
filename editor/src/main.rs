mod editor_scene;
mod egui_layer;
mod fly_camera;
mod menu_scene;
mod picking;
mod project;
mod recent_projects;
mod ui;

use libdqg::game::GameBuilder;
use libdqg::types::Color;
use menu_scene::MenuScene;

fn main() {
    let mut game = GameBuilder::new(Box::new(MenuScene::new()))
        .title("libDQG Editor".into())
        .size(1280, 800)
        .integrated_titlebar(true)
        .clear_color(Color::new(0.05, 0.05, 0.07, 1.0))
        .resizable(true)
        .build();
    game.run();
}
