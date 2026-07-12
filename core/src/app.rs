use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalSize, Size};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::window::{CursorIcon, ResizeDirection, Window, WindowAttributes, WindowButtons, WindowId};
use crate::scene::{Scene, SceneManager};
use crate::renderer::Renderer;
use crate::input::InputState;
use crate::titlebar::{TitleBar, TitleBarButton, TitleBarHit};
use crate::types::Color;

const RESIZE_BORDER: f32 = 6.0;

fn resize_direction(x: f32, y: f32, w: f32, h: f32) -> Option<ResizeDirection> {
    let left = x <= RESIZE_BORDER;
    let right = x >= w - RESIZE_BORDER;
    let top = y <= RESIZE_BORDER;
    let bottom = y >= h - RESIZE_BORDER;

    match (left, right, top, bottom) {
        (true, _, true, _) => Some(ResizeDirection::NorthWest),
        (_, true, true, _) => Some(ResizeDirection::NorthEast),
        (true, _, _, true) => Some(ResizeDirection::SouthWest),
        (_, true, _, true) => Some(ResizeDirection::SouthEast),
        (true, _, _, _) => Some(ResizeDirection::West),
        (_, true, _, _) => Some(ResizeDirection::East),
        (_, _, true, _) => Some(ResizeDirection::North),
        (_, _, _, true) => Some(ResizeDirection::South),
        _ => None,
    }
}

pub(crate) struct App<'a> {
    title: String,
    width: u32,
    height: u32,
    window: Option<std::sync::Arc<Window>>,
    scene_manager: SceneManager,
    renderer: Option<Renderer<'a>>,
    input_state: InputState,
    frame_start: Instant,
    titlebar: Option<TitleBar>,
    resizable: bool,
    cursor_pos: (f32, f32),
}

impl<'a> App<'a> {
    pub fn new(title: String, width: u32, height: u32, integrated_titlebar: bool, resizable: bool) -> Self {
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
            titlebar: integrated_titlebar.then(|| TitleBar::new(resizable)),
            resizable: resizable,
            cursor_pos: (0.0, 0.0),
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
        attrs.resizable = self.resizable;
        attrs.enabled_buttons = self.resizable.then(
            || WindowButtons::all()
        ).unwrap_or(WindowButtons::MINIMIZE |WindowButtons::CLOSE);
        if self.titlebar.is_some() {
            attrs.decorations = false;
        }
        match event_loop.create_window(attrs) {
            Ok(window) => {
                crate::platform::round_window_corners(&window);
                self.window = Some(std::sync::Arc::new(window));
            },
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
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = (position.x as f32, position.y as f32);
                if let Some(window) = self.window.as_ref() {
                    let size = window.inner_size();
                    if let Some(ref mut titlebar) = self.titlebar {
                        titlebar.update_hover(self.cursor_pos.0, self.cursor_pos.1, size.width as f32);
                    }
                    let cursor = if self.resizable {
                        match resize_direction(self.cursor_pos.0, self.cursor_pos.1, size.width as f32, size.height as f32) {
                            Some(dir) => CursorIcon::from(dir),
                            None => CursorIcon::Default,
                        }
                    } else {
                        CursorIcon::Default
                    };
                    window.set_cursor(cursor);
                }
            },
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                if let Some(window) = self.window.clone() {
                    let size = window.inner_size();
                    let (x, y) = self.cursor_pos;
                    let dir = if self.resizable {
                        resize_direction(x, y, size.width as f32, size.height as f32)
                    } else {
                        None
                    };
                    if let Some(dir) = dir {
                        let _ = window.drag_resize_window(dir);
                    } else {
                        if let Some(ref titlebar) = self.titlebar {
                            match titlebar.hit_test(x, y, size.width as f32) {
                                TitleBarHit::Button(TitleBarButton::Close) => event_loop.exit(),
                                TitleBarHit::Button(TitleBarButton::Minimize) => window.set_minimized(true),
                                TitleBarHit::Button(TitleBarButton::Maximize) => window.set_maximized(!window.is_maximized()),
                                TitleBarHit::Drag => { let _ = window.drag_window(); },
                                TitleBarHit::None => {},
                            }
                        }
                    }
                }
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
                    let titlebar = &self.titlebar;
                    let window = self.window.as_ref().unwrap();
                    let maximized = window.is_maximized();
                    let screen_w = window.inner_size().width;
                    renderer.render(clear_color, |pass| {
                        self.scene_manager.render(pass);
                        match titlebar.as_ref() {
                            Some(tb) => tb.render(pass, screen_w, maximized),
                            None => {},
                        }
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
