use crate::{
    input::{InputState, MouseState},
    renderer::{DrawPass, Renderer},
};

pub trait Scene {
    fn update(&mut self, delta_time: f32, input_state: &InputState, mouse_state: &MouseState, renderer: Option<&mut Renderer>) -> SceneTransition;
    fn render(&mut self, pass: &mut DrawPass);

    /// Receives every raw `winit` window event, ahead of any `App`-level handling. Default is a
    /// no-op; override to feed events into something like `egui-winit` that needs the raw stream.
    fn on_window_event(&mut self, _event: &winit::event::WindowEvent) {}

    /// Called after the main render pass has ended but before the frame is submitted, with the
    /// same encoder/target view so a second pass can be recorded on top (e.g. compositing an
    /// `egui` UI). Default is a no-op.
    fn render_overlay(&mut self, _device: &wgpu::Device, _queue: &wgpu::Queue, _encoder: &mut wgpu::CommandEncoder, _view: &wgpu::TextureView) {}
}

pub enum SceneTransition {
    None,
    Push(Box<dyn Scene>),
    Pop,
    Replace(Box<dyn Scene>),
    Next,
    Previous,
    Quit,
}

pub struct SceneManager {
    scenes: Vec<Box<dyn Scene>>,
    current_scene_index: usize,
}

impl SceneManager {
    pub fn new() -> Self {
        Self { scenes: Vec::new(), current_scene_index: 0 }
    }

    pub fn add_scene(&mut self, scene: Box<dyn Scene>) {
        self.scenes.push(scene);
    }

    pub fn update(&mut self, delta_time: f32, input_state: &InputState, mouse_state: &MouseState, renderer: Option<&mut Renderer>) {
        if let Some(current_scene) = self.scenes.get_mut(self.current_scene_index) {
            let transition = current_scene.update(delta_time, input_state, mouse_state, renderer);
            match transition {
                SceneTransition::None => (),
                SceneTransition::Push(new_scene) => self.scenes.push(new_scene),
                SceneTransition::Pop => {
                    self.scenes.pop();
                    self.current_scene_index %= self.scenes.len(); // Ensure the index is valid after popping
                }
                SceneTransition::Replace(new_scene) => {
                    self.scenes[self.current_scene_index] = new_scene;
                }
                SceneTransition::Quit => {
                    todo!(); // Handle quitting the game, e.g., by signaling the app to exit
                }
                SceneTransition::Next => {
                    self.current_scene_index = (self.current_scene_index + 1) % self.scenes.len();
                }
                SceneTransition::Previous => {
                    self.current_scene_index = (self.current_scene_index + self.scenes.len() - 1) % self.scenes.len();
                }
            }
        }
    }

    pub fn render(&mut self, pass: &mut DrawPass) {
        if let Some(current_scene) = self.scenes.get_mut(self.current_scene_index) {
            current_scene.render(pass);
        }
    }

    pub fn on_window_event(&mut self, event: &winit::event::WindowEvent) {
        if let Some(current_scene) = self.scenes.get_mut(self.current_scene_index) {
            current_scene.on_window_event(event);
        }
    }

    pub fn render_overlay(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        if let Some(current_scene) = self.scenes.get_mut(self.current_scene_index) {
            current_scene.render_overlay(device, queue, encoder, view);
        }
    }
}
