use libdqg::{camera::Camera, types::KeyCode};

pub struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        }
    }

    pub fn handle_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::ArrowUp => {
                self.is_forward_pressed = true;
                true
            }
            KeyCode::ArrowLeft => {
                self.is_left_pressed = true;
                true
            }
            KeyCode::ArrowDown => {
                self.is_backward_pressed = true;
                true
            }
            KeyCode::ArrowRight => {
                self.is_right_pressed = true;
                true
            }
            _ => false,
        }
    }

    pub fn update_camera(&mut self, delta_time: f32, camera: &mut Camera) {
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.length();

        // Prevents glitching when the camera gets too close to the
        // center of the scene.
        if self.is_forward_pressed && forward_mag > self.speed * delta_time {
            camera.eye += forward_norm * self.speed * delta_time;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed * delta_time;
        }

        let right = forward_norm.cross(camera.up);

        // Redo radius calc in case the forward/backward is pressed.
        let forward = camera.target - camera.eye;
        let forward_mag = forward.length();

        if self.is_right_pressed {
            // Rescale the distance between the target and the eye so
            // that it doesn't change. The eye, therefore, still
            // lies on the circle made by the target and eye.
            camera.eye = camera.target - (forward + right * self.speed * delta_time).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            camera.eye = camera.target - (forward - right * self.speed * delta_time).normalize() * forward_mag;
        }

        self.is_backward_pressed = false;
        self.is_forward_pressed = false;
        self.is_left_pressed = false;
        self.is_right_pressed = false;
    }
}
