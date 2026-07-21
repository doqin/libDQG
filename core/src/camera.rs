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

pub struct Camera {
    pub eye: glam::Vec3, // Camera position
    pub target: glam::Vec3, // Point the camera is looking at
    pub up: glam::Vec3, // Camera up direction
    pub aspect: f32,
    /// Vertical field of view, in degrees.
    pub fov: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> glam::Mat4 {
        let view = glam::camera::rh::view::look_at_mat4(self.eye, self.target, self.up);
        let projection = glam::camera::rh::proj::directx::perspective(self.fov.to_radians(), self.aspect, self.znear, self.zfar);
        projection * view
    }
}
