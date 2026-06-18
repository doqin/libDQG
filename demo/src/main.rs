use libdqg::game::{Game, run_game};

struct MyGame {}

impl Game for MyGame {
    fn init(&mut self) {
        // Initialization code here
    }

    fn update(&mut self) {
        // Update code here
    }

    fn render(&mut self) {
        // Render code here
    }
}

fn main() {
    let game = MyGame {};
    run_game(game);
}
