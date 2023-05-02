use bytemuck::{Pod, Zeroable};
use wgpu::VertexBufferLayout;

use crate::gfx::camera::Camera;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
}

impl Vertex {
    pub fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            }],
        }
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

        unsafe {
            STARTED_MS = std::time::SystemTime::now();
        }

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

static mut STARTED_MS: std::time::SystemTime = std::time::SystemTime::UNIX_EPOCH;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WaveParams {
    pub dim_x: u32,
    pub dim_y: u32,
    pub size: f32,
    pub speed: f32,
    pub height: f32,
    pub noise: f32,
    time: u32,
}

impl WaveParams {
    pub fn new() -> Self {
        unsafe {
            STARTED_MS = std::time::SystemTime::now();
        }

        Self {
            dim_x: 100,
            dim_y: 80,
            size: 18.0,
            speed: 1.0,
            height: 15.0,
            noise: 4.0,
            time: 0,
        }
    }

    pub fn update_time(&mut self) {
        self.time = unsafe {
            std::time::SystemTime::now()
                .duration_since(STARTED_MS)
                .unwrap()
                .as_millis() as u32
        };
    }

    pub fn area(&self) -> u32 {
        self.dim_x * self.dim_y
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WaveRenderParams {
    pub colour: [f32; 3],
    spacer: f32,
}

impl WaveRenderParams {
    pub fn new(colour: [f32; 3]) -> WaveRenderParams {
        WaveRenderParams {
            colour: colour,
            spacer: 0.0,
        }
    }
}
