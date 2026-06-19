use crate::{
    input::InputState, 
    renderer::Renderer
};

pub trait Scene {
    fn update(&mut self, delta_time: f32, input_state: &InputState) -> SceneTransition;
    fn render(&mut self, renderer: &mut Renderer);
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

    pub fn update(&mut self, delta_time: f32, input_state: &InputState) {
        if let Some(current_scene) = self.scenes.get_mut(self.current_scene_index) {
            let transition = current_scene.update(delta_time, input_state);
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

    pub fn render(&mut self, renderer: &mut Renderer) {
        if let Some(current_scene) = self.scenes.get_mut(self.current_scene_index) {
            current_scene.render(renderer);
        }
    }
}