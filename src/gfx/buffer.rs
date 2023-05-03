use crate::gfx::camera::Camera;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraMatrix {
    proj: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
}

impl CameraMatrix {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;

        Self {
            proj: cgmath::Matrix4::identity().into(),
            view: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.proj = camera.build_projection_matrix().into();
        self.view = camera.build_view_matrix().into();
    }
}
