use libdqg::camera::Camera;
use libdqg::glam;
use libdqg::input::{InputState, MouseState};
use libdqg::types::KeyCode;

/// WASD + middle-mouse-drag look + wheel zoom, orbiting `target` at a fixed distance rather than
/// flying freely — simplest thing that lets you look at a spawned entity from any angle.
pub struct FlyCamera {
    yaw: f32,
    pitch: f32,
    /// Orbit pivot. `Camera` itself only stores position + yaw/pitch, not a target, so this has
    /// to live here instead.
    target: glam::Vec3,
    distance: f32,
    move_speed: f32,
    look_speed: f32,
    zoom_speed: f32,
}

impl FlyCamera {
    pub fn new(distance: f32) -> Self {
        Self {
            yaw: -90.0_f32.to_radians(),
            pitch: -20.0_f32.to_radians(),
            target: glam::Vec3::ZERO,
            distance,
            move_speed: 4.0,
            look_speed: 0.005,
            zoom_speed: 0.5,
        }
    }

    pub fn update(&mut self, delta_time: f32, input: &InputState, mouse: &MouseState, camera: &mut Camera, mouse_delta: (f32, f32)) {
        if mouse.is_button_held(winit::event::MouseButton::Middle) {
            self.yaw += mouse_delta.0 * self.look_speed;
            self.pitch = (self.pitch - mouse_delta.1 * self.look_speed).clamp(-1.5, 1.5);
        }

        self.distance = (self.distance - mouse.wheel_delta() * self.zoom_speed).max(0.5);

        camera.yaw = self.yaw;
        camera.pitch = self.pitch;
        let forward = camera.forward();
        let right = forward.cross(glam::Vec3::Y).normalize();

        let mut move_delta = glam::Vec3::ZERO;
        if input.is_key_held(KeyCode::KeyW) {
            move_delta += forward;
        }
        if input.is_key_held(KeyCode::KeyS) {
            move_delta -= forward;
        }
        if input.is_key_held(KeyCode::KeyA) {
            move_delta -= right;
        }
        if input.is_key_held(KeyCode::KeyD) {
            move_delta += right;
        }
        if move_delta != glam::Vec3::ZERO {
            self.target += move_delta.normalize() * self.move_speed * delta_time;
        }

        camera.position = self.target - forward * self.distance;
    }
}
