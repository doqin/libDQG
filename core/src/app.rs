use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalSize, Size};
use winit::event::WindowEvent;
use winit::window::{Window, WindowAttributes, WindowId};
use crate::scene::{Scene, SceneManager};
use crate::renderer::Renderer;
use crate::input::InputState;
use crate::types::Color;

pub struct App<'a> {
    title: String,
    width: u32,
    height: u32,
    window: Option<std::sync::Arc<Window>>,
    scene_manager: SceneManager,
    renderer: Option<Renderer<'a>>,
    input_state: InputState,
    frame_start: Instant,
}

impl<'a> App<'a> {
    pub fn new(title: String, width: u32, height: u32) -> Self {
        let scene_manager = SceneManager::new();
        Self {
            title,
            width,
            height,
            window: None,
            scene_manager,
            renderer: None,
            input_state: InputState::new(),
            frame_start: Instant::now(),
        }
    }

    pub fn add_scene(&mut self, scene: Box<dyn Scene>) {
        self.scene_manager.add_scene(scene);
    }
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let mut attrs = WindowAttributes::default();
        attrs.title = self.title.clone();
        attrs.inner_size = Some(Size::Physical(PhysicalSize::new(self.width, self.height)));
        match event_loop.create_window(attrs) {
            Ok(window) => self.window = Some(std::sync::Arc::new(window)),
            Err(e) => panic!("Failed to create window: {:?}", e),
        };
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
                let frame_time = self.frame_start.elapsed();
                self.frame_start = Instant::now();
                // Handle redraw here
                self.scene_manager.update(frame_time.as_secs_f32(), &self.input_state);
                if let Some(renderer) = self.renderer.as_mut() {
                    let clear_color = Color::new(0.1, 0.2, 0.3, 1.0);
                    renderer.render(clear_color, |pass| {
                        self.scene_manager.render(pass);
                    });
                }
                self.input_state.clear_frame_states();
                // Request another redraw
                self.window.as_ref().unwrap().request_redraw();
            },
            _ => (),
        }
    }
}