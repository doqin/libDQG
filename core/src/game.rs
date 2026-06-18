use winit::event_loop::EventLoop;
use crate::scene::Scene;

use crate::app::App;

pub fn run_game(initial_scene: Box<dyn Scene>) {
    let mut app = App::new(initial_scene);
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    match event_loop.run_app(&mut app) {
        Ok(_) => println!("Application exited successfully."),
        Err(e) => eprintln!("Application error: {:?}", e),
    }
}