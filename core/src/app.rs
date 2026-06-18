use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::window::{Window, WindowId};
use crate::scene::{Scene, SceneManager};
use crate::renderer::Renderer;
use crate::input::InputState;

pub struct App<'a> {
    window: Option<std::sync::Arc<Window>>,
    scene_manager: SceneManager,
    renderer: Option<Renderer<'a>>,
    input_state: InputState,
}

impl<'a> App<'a> {
    pub fn new(scene: Box<dyn Scene>) -> Self {
        let mut scene_manager = SceneManager::new();
        scene_manager.add_scene(scene);

        Self {
            window: None,
            scene_manager,
            renderer: None,
            input_state: InputState::new(),
        }
    }
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        match event_loop.create_window(Window::default_attributes()) {
            Ok(window) => self.window = Some(std::sync::Arc::new(window)),
            Err(e) => panic!("Failed to create window: {:?}", e),
        }
        let renderer = pollster::block_on(Renderer::new(self.window.as_ref().unwrap().clone()));
        self.renderer = Some(renderer);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                self.input_state.handle_event(&event);
            }
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            },
            WindowEvent::Resized(physical_size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    let window = self.window.as_ref().unwrap();
                    renderer.resize(physical_size);
                    window.request_redraw();
                }
            },
            WindowEvent::RedrawRequested => {
                // Handle redraw here
                self.scene_manager.update(&self.input_state);
                self.scene_manager.render(&mut self.renderer.as_mut().unwrap());
                self.input_state.clear_frame_states();
                // Request another redraw
                self.window.as_ref().unwrap().request_redraw();
            },
            _ => (),
        }
    }
}