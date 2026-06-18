use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::window::{Window, WindowId};
use crate::game::Game;

#[derive(Default)]
pub struct App<G> where G: Game {
    window: Option<Window>,
    game: G,
}

impl<G> App<G> where G: Game {
    pub fn new(mut game: G) -> Self {
        game.init();
        Self {
            window: None,
            game: game,
        }
    }
}

impl<G> ApplicationHandler for App<G> where G: Game {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        match event_loop.create_window(Window::default_attributes()) {
            Ok(window) => self.window = Some(window),
            Err(e) => panic!("Failed to create window: {:?}", e),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            },
            WindowEvent::RedrawRequested => {
                // Handle redraw here
                self.game.update();
                self.game.render();
                // Request another redraw
                self.window.as_ref().unwrap().request_redraw();
            },
            _ => (),
        }
    }
}