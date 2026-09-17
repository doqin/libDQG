#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub(crate) fn new() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    pub(crate) fn update(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().to_cols_array_2d();
    }
}

#[derive(Clone, Copy)]
pub struct Camera {
    pub position: glam::Vec3,
    /// Radians, rotation around world +Y.
    pub yaw: f32,
    /// Radians. Callers should keep this in roughly (-PI/2, PI/2) to avoid the view flipping
    /// through the poles — `up` is fixed to world +Y, so there's no roll to fall back on there.
    pub pitch: f32,
    pub aspect: f32,
    /// Vertical field of view, in degrees.
    pub fov: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn forward(&self) -> glam::Vec3 {
        glam::Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
    }

    /// Extracts yaw/pitch (no roll) from a normalized direction vector — the inverse of
    /// `forward()`. Shared by `look_at` and by `CameraComponent::derive_camera`, which both need
    /// to turn a direction into this camera's angle representation.
    pub fn yaw_pitch_from_forward(forward: glam::Vec3) -> (f32, f32) {
        (forward.z.atan2(forward.x), forward.y.clamp(-1.0, 1.0).asin())
    }

    /// Points the camera at `target` from its current `position`, for call sites that think in
    /// terms of "look at this point" rather than angles.
    pub fn look_at(&mut self, target: glam::Vec3) {
        let (yaw, pitch) = Self::yaw_pitch_from_forward((target - self.position).normalize());
        self.yaw = yaw;
        self.pitch = pitch;
    }

    pub fn build_view_projection_matrix(&self) -> glam::Mat4 {
        let view = glam::camera::rh::view::look_to_mat4(self.position, self.forward(), glam::Vec3::Y);
        let projection = glam::camera::rh::proj::directx::perspective(self.fov.to_radians(), self.aspect, self.znear, self.zfar);
        projection * view
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_matches_identity_yaw_pitch() {
        let camera = Camera { position: glam::Vec3::ZERO, yaw: 0.0, pitch: 0.0, aspect: 1.0, fov: 45.0, znear: 0.1, zfar: 100.0 };
        assert!(camera.forward().abs_diff_eq(glam::Vec3::X, 1e-5));
    }

    #[test]
    fn look_at_and_forward_round_trip() {
        let mut camera = Camera { position: glam::Vec3::new(1.0, 2.0, 3.0), yaw: 0.0, pitch: 0.0, aspect: 1.0, fov: 45.0, znear: 0.1, zfar: 100.0 };
        let target = glam::Vec3::new(-4.0, 5.0, 10.0);

        camera.look_at(target);

        let expected_dir = (target - camera.position).normalize();
        assert!(camera.forward().abs_diff_eq(expected_dir, 1e-4));
    }
}
