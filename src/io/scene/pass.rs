use serde::Deserialize;
use wgpu::{Face, FrontFace, PolygonMode, PrimitiveTopology, ShaderStages};

use super::resource::BufferVertex;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum RenderPass {
    Compute {
        label: Option<String>,
        pipeline: ComputePipeline,
        workgroups: [u32; 3],
    },
    Render {
        label: Option<String>,
        pipeline: RenderPipeline,
        clear: Option<RenderClear>,
        draw: Vec<RenderDraw>,
    },
}

#[derive(Debug, Deserialize)]
pub struct ComputePipeline {
    pub shader: String,
    pub bindings: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RenderPipeline {
    pub shader_vertex: String,
    pub bindings: Option<Vec<String>>,
    pub bindings_visibility: Option<Vec<RenderPipelineBindingVisibility>>,
    pub shader_fragment: Option<String>,
    pub topology: RenderPipelineTopology,
    pub polygon_mode: RenderPipelinePolygonMode,
    pub front_face: RenderPipelineFrontFace,
    pub cull_mode: RenderPipelineCullMode,
    pub vertex: BufferVertex,
}

#[derive(Debug, Deserialize)]
pub enum RenderPipelineBindingVisibility {
    None,
    Vertex,
    Fragment,
    VertexFragment,
    Compute,
}

impl RenderPipelineBindingVisibility {
    pub fn as_wgpu(&self) -> ShaderStages {
        match self {
            RenderPipelineBindingVisibility::None => ShaderStages::NONE,
            RenderPipelineBindingVisibility::Vertex => ShaderStages::VERTEX,
            RenderPipelineBindingVisibility::Fragment => ShaderStages::FRAGMENT,
            RenderPipelineBindingVisibility::VertexFragment => ShaderStages::VERTEX_FRAGMENT,
            RenderPipelineBindingVisibility::Compute => ShaderStages::COMPUTE,
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum RenderPipelineTopology {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
}

impl RenderPipelineTopology {
    pub fn as_wgpu(&self) -> PrimitiveTopology {
        match self {
            RenderPipelineTopology::PointList => PrimitiveTopology::PointList,
            RenderPipelineTopology::LineList => PrimitiveTopology::LineList,
            RenderPipelineTopology::LineStrip => PrimitiveTopology::LineStrip,
            RenderPipelineTopology::TriangleList => PrimitiveTopology::TriangleList,
            RenderPipelineTopology::TriangleStrip => PrimitiveTopology::TriangleStrip,
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum RenderPipelinePolygonMode {
    Fill,
    Line,
    Point,
}

impl RenderPipelinePolygonMode {
    pub fn as_wgpu(&self) -> PolygonMode {
        match self {
            RenderPipelinePolygonMode::Fill => PolygonMode::Fill,
            RenderPipelinePolygonMode::Line => PolygonMode::Line,
            RenderPipelinePolygonMode::Point => PolygonMode::Point,
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum RenderPipelineFrontFace {
    Ccw,
    Cw,
}

impl RenderPipelineFrontFace {
    pub fn as_wgpu(&self) -> FrontFace {
        match self {
            RenderPipelineFrontFace::Ccw => FrontFace::Ccw,
            RenderPipelineFrontFace::Cw => FrontFace::Cw,
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum RenderPipelineCullMode {
    Front,
    Back,
    None,
}

impl RenderPipelineCullMode {
    pub fn as_wgpu(&self) -> Option<Face> {
        match self {
            RenderPipelineCullMode::Front => Some(Face::Front),
            RenderPipelineCullMode::Back => Some(Face::Back),
            RenderPipelineCullMode::None => None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RenderClear {
    pub colour: Option<String>,
    pub depth_stencil: Option<()>,
}

#[derive(Debug, Deserialize)]
pub struct RenderDraw {
    pub vertex_buffer: Option<String>,
}
