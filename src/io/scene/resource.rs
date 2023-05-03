use serde::Deserialize;
use wgpu::{BufferAddress, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode};

/*
 * TODO resource types:
 *   load models into buffers?
 *   textures
 *   fonts?
 */

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Resource {
    Buffer {
        label: Option<String>,
        size: usize,
        storage: Option<BufferStorage>,
        vertex: Option<BufferVertex>,
    },
    Camera {
        projection: CameraProjection,
        position: [f64; 3],
        look_at: [f64; 3],
    },
    Shader {
        src: String,
        label: Option<String>,
        main: Option<String>,
        vertex_main: Option<String>,
        fragment_main: Option<String>,
    },
    Uniform {
        label: Option<String>,
        values: Vec<String>,
    },
}

#[derive(Clone, Debug, Deserialize)]
pub struct BufferStorage {
    pub storage_type: BufferStorageType,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BufferStorageType {
    Read,
    ReadWrite,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BufferVertex {
    pub stride: usize,
    pub step: Option<BufferVertexStep>,
    pub attributes: Vec<BufferVertexAttribute>,
}

impl BufferVertex {
    #[allow(dead_code)]
    pub fn compatible(&self, _other: &BufferVertex) -> bool {
        todo!("compare vertex layouts to see if they are interchangable")
        // will need to compare
        // -> equal stride
        // -> equal step (or other step == None)
        // -> defined attributes match
        // self will be the layout we want, other is the layout we are testing
    }

    pub fn attributes(&self) -> Vec<VertexAttribute> {
        self.attributes
            .iter()
            .map(|attribute| attribute.as_wgpu())
            .collect()
    }

    pub fn desc<'a>(&self, attributes: &'a [VertexAttribute]) -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: self.stride as BufferAddress,
            step_mode: self
                .step
                .as_ref()
                .unwrap_or(&BufferVertexStep::Vertex)
                .as_wgpu(),
            attributes,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BufferVertexStep {
    Vertex,
    Instance,
}

impl BufferVertexStep {
    pub fn as_wgpu(&self) -> VertexStepMode {
        match self {
            BufferVertexStep::Vertex => VertexStepMode::Vertex,
            BufferVertexStep::Instance => VertexStepMode::Instance,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct BufferVertexAttribute {
    pub offset: usize,
    pub location: usize,
    pub format: BufferVertexAttributeFormat,
}

impl BufferVertexAttribute {
    pub fn as_wgpu(&self) -> VertexAttribute {
        VertexAttribute {
            offset: self.offset as u64,
            format: self.format.as_wgpu(),
            shader_location: self.location as u32,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub enum BufferVertexAttributeFormat {
    Float32x3,
}

impl BufferVertexAttributeFormat {
    pub fn as_wgpu(&self) -> VertexFormat {
        match self {
            Self::Float32x3 => VertexFormat::Float32x3,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraProjection {
    Perspective,
    Orthographic,
}
