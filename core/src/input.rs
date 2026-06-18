use std::collections::HashSet;

pub struct InputState {
    keys_held: HashSet<crate::types::KeyCode>,
    keys_pressed: HashSet<crate::types::KeyCode>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_held: HashSet::new(),
            keys_pressed: HashSet::new(),
        }
    }

    pub fn handle_event(&mut self, event: &winit::event::KeyEvent) {
        if let winit::keyboard::PhysicalKey::Code(keycode) = event.physical_key {
            let native_keycode = match crate::types::KeyCode::from_winit_keycode(keycode) {
                Some(keycode) => keycode,
                None => todo!("Handle unmapped keycode: {:?}", keycode),
            };
            if event.state.is_pressed() {
                if !self.keys_held.contains(&native_keycode) {
                    self.keys_pressed.insert(native_keycode);
                }
                self.keys_held.insert(native_keycode);
            } else {
                self.keys_held.remove(&native_keycode);
            }
        }
    }

    pub fn clear_frame_states(&mut self) {
        self.keys_pressed.clear();
    }

    pub fn is_key_held(&self, keycode: crate::types::KeyCode) -> bool {
        self.keys_held.contains(&keycode)
    }

    pub fn is_key_pressed(&self, keycode: crate::types::KeyCode) -> bool {
        self.keys_pressed.contains(&keycode)
    }
}