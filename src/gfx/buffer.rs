use crate::gfx::camera::Camera;

static mut STARTED_MS: std::time::SystemTime = std::time::SystemTime::UNIX_EPOCH;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Time {
    time: u32,
    dt: f32,
}

impl Time {
    pub fn new() -> Self {
        unsafe {
            STARTED_MS = std::time::SystemTime::now();
        }

        Time { time: 0, dt: 0.0 }
    }

    pub fn update_time(&mut self, dt: f64) {
        self.time = unsafe {
            std::time::SystemTime::now()
                .duration_since(STARTED_MS)
                .unwrap()
                .as_millis() as u32
        };
        self.dt = dt as f32;
    }
}

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
