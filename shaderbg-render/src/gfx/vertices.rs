#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex2dTex2f {
    position: [f32; 2],
    texture: [f32; 2],
}

pub const VERTICES_QUAD: &[Vertex2dTex2f] = &[
    Vertex2dTex2f {
        position: [-1.0, -1.0],
        texture: [0.0, 0.0],
    },
    Vertex2dTex2f {
        position: [-1.0, 1.0],
        texture: [0.0, 1.0],
    },
    Vertex2dTex2f {
        position: [1.0, -1.0],
        texture: [1.0, 0.0],
    },
    Vertex2dTex2f {
        position: [1.0, -1.0],
        texture: [1.0, 0.0],
    },
    Vertex2dTex2f {
        position: [-1.0, 1.0],
        texture: [0.0, 1.0],
    },
    Vertex2dTex2f {
        position: [1.0, 1.0],
        texture: [1.0, 1.0],
    },
];
