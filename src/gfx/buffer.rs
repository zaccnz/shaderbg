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

pub type Index = u16;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraMatrix {
    proj: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    time: u32,
    spacer: [u32; 3],
}

static mut STARTED_MS: std::time::SystemTime = std::time::SystemTime::UNIX_EPOCH;

impl CameraMatrix {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;

        unsafe {
            STARTED_MS = std::time::SystemTime::now();
        }

        Self {
            proj: cgmath::Matrix4::identity().into(),
            view: cgmath::Matrix4::identity().into(),
            time: 0,
            spacer: [0; 3],
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.proj = camera.build_projection_matrix().into();
        self.view = camera.build_view_matrix().into();
        self.time = unsafe {
            std::time::SystemTime::now()
                .duration_since(STARTED_MS)
                .unwrap()
                .as_millis() as u32
        };
    }
}
