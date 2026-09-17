use std::collections::HashSet;

use crate::types::KeyCode;

pub struct InputState {
    keys_held: HashSet<KeyCode>,
    keys_pressed: HashSet<KeyCode>,
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
            let native_keycode = match KeyCode::from_winit_keycode(keycode) {
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

    pub fn is_key_held(&self, keycode: KeyCode) -> bool {
        self.keys_held.contains(&keycode)
    }

    pub fn is_key_pressed(&self, keycode: KeyCode) -> bool {
        self.keys_pressed.contains(&keycode)
    }

    pub fn keys_pressed(&self) -> impl Iterator<Item = KeyCode> + '_ {
        self.keys_pressed.iter().copied()
    }

    pub fn keys_held(&self) -> impl Iterator<Item = KeyCode> + '_ {
        self.keys_held.iter().copied()
    }

    /// Builds an [`InputState`] with a fixed held/pressed set directly, bypassing
    /// [`InputState::handle_event`]'s `winit::event::KeyEvent` plumbing — there's no lightweight
    /// way to construct a real one outside of an actual window/event loop, and tests (e.g.
    /// `core::scripting`'s input-scripting tests) just need specific keys reported as
    /// held/pressed.
    #[cfg(test)]
    pub(crate) fn for_test(held: impl IntoIterator<Item = KeyCode>, pressed: impl IntoIterator<Item = KeyCode>) -> Self {
        Self { keys_held: held.into_iter().collect(), keys_pressed: pressed.into_iter().collect() }
    }
}

/// Per-frame mouse state: cursor position, button press/hold, and wheel motion.
///
/// Mirrors [`InputState`] but is tracked separately since it's fed from a different set of
/// `winit` events and updated by different call sites in `App`.
pub struct MouseState {
    position: (f32, f32),
    buttons_held: HashSet<winit::event::MouseButton>,
    buttons_pressed: HashSet<winit::event::MouseButton>,
    wheel_delta: f32,
}

impl MouseState {
    pub fn new() -> Self {
        Self {
            position: (0.0, 0.0),
            buttons_held: HashSet::new(),
            buttons_pressed: HashSet::new(),
            wheel_delta: 0.0,
        }
    }

    pub fn set_position(&mut self, x: f32, y: f32) {
        self.position = (x, y);
    }

    pub fn handle_button(&mut self, button: winit::event::MouseButton, state: winit::event::ElementState) {
        if state.is_pressed() {
            if !self.buttons_held.contains(&button) {
                self.buttons_pressed.insert(button);
            }
            self.buttons_held.insert(button);
        } else {
            self.buttons_held.remove(&button);
        }
    }

    pub fn handle_wheel(&mut self, delta: winit::event::MouseScrollDelta) {
        self.wheel_delta += match delta {
            winit::event::MouseScrollDelta::LineDelta(_, y) => y,
            winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 120.0,
        };
    }

    pub fn clear_frame_states(&mut self) {
        self.buttons_pressed.clear();
        self.wheel_delta = 0.0;
    }

    pub fn position(&self) -> (f32, f32) {
        self.position
    }

    pub fn is_button_held(&self, button: winit::event::MouseButton) -> bool {
        self.buttons_held.contains(&button)
    }

    pub fn is_button_pressed(&self, button: winit::event::MouseButton) -> bool {
        self.buttons_pressed.contains(&button)
    }

    pub fn buttons_held(&self) -> impl Iterator<Item = winit::event::MouseButton> + '_ {
        self.buttons_held.iter().copied()
    }

    pub fn buttons_pressed(&self) -> impl Iterator<Item = winit::event::MouseButton> + '_ {
        self.buttons_pressed.iter().copied()
    }

    pub fn wheel_delta(&self) -> f32 {
        self.wheel_delta
    }

    /// Builds a [`MouseState`] with fixed position/held/pressed/wheel values directly, the mouse
    /// counterpart to [`InputState::for_test`] — see its doc comment for why this exists instead
    /// of driving one through real `winit` events.
    #[cfg(test)]
    pub(crate) fn for_test(
        position: (f32, f32),
        held: impl IntoIterator<Item = winit::event::MouseButton>,
        pressed: impl IntoIterator<Item = winit::event::MouseButton>,
        wheel_delta: f32,
    ) -> Self {
        Self { position, buttons_held: held.into_iter().collect(), buttons_pressed: pressed.into_iter().collect(), wheel_delta }
    }
}
