use crate::gfx::camera::Camera;

// static mut STARTED_MS: std::time::SystemTime = std::time::SystemTime::UNIX_EPOCH;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Time {
    pub time: u32,
    pub dt: f32,
}

impl Time {
    pub fn new() -> Self {
        Time { time: 0, dt: 0.0 }
    }

    pub fn update_time(&mut self, now: u32, dt: f64) {
        self.time = now;
        self.dt = dt as f32;
    }
}

impl Default for Time {
    fn default() -> Self {
        Time::new()
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShaderToy {
    resolution: [f32; 3],
    _spacer: f32,
    time: f32,
    time_delta: f32,
    _spacer2: f32,
    _spacer3: f32,
    mouse: [f32; 4],
}

impl ShaderToy {
    pub fn new() -> Self {
        ShaderToy {
            resolution: [800.0, 600.0, 0.0],
            _spacer: 0.0,
            time: 0.0,
            time_delta: 0.0,
            _spacer2: 0.0,
            _spacer3: 0.0,
            mouse: [0.0; 4],
        }
    }

    pub fn update(&mut self, now: u32, dt: f64, width: u32, height: u32) {
        self.time = (now as f32) / 1000.0;
        self.time_delta = dt as f32;
        self.resolution = [width as f32, height as f32, 0.0];
    }
}

impl Default for ShaderToy {
    fn default() -> Self {
        ShaderToy::new()
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

impl Default for CameraMatrix {
    fn default() -> Self {
        CameraMatrix::new()
    }
}
