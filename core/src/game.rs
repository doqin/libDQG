use winit::event_loop::EventLoop;

use crate::app::App;

pub trait Game {
    fn init(&mut self);
    fn update(&mut self);
    fn render(&mut self);
}

pub fn run_game<G: Game + 'static>(game: G) {
    let mut app = App::new(game);
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    match event_loop.run_app(&mut app) {
        Ok(_) => println!("Application exited successfully."),
        Err(e) => eprintln!("Application error: {:?}", e),
    }
}