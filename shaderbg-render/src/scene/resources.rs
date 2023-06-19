/*
 * Stores all of the scene resources (per thread)
 * Also handles rendering of the scene
 */

use cgmath::Point3;
use std::{borrow::Cow, collections::HashMap, mem, ops::Deref, str::Utf8Error};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendState, Buffer, BufferAddress, BufferBindingType,
    BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder, ComputePipeline,
    ComputePipelineDescriptor, Device, FragmentState, PipelineLayoutDescriptor, PrimitiveState,
    Queue, RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor,
    TextureFormat, TextureView, VertexAttribute, VertexBufferLayout, VertexState,
};

use crate::{
    gfx::{
        buffer::{CameraMatrix, ShaderToy, Time},
        camera::Camera,
        vertices::VERTICES_QUAD,
    },
    scene::{
        io::{
            pass::{RenderClear, RenderDraw, RenderPass, RenderPipelineBindingVisibility},
            resource::{
                BufferStorage, BufferStorageType, BufferVertex, BufferVertexAttribute,
                BufferVertexAttributeFormat, BufferVertexStep, Resource, ShaderFormat,
            },
        },
        Scene, Setting,
    },
};

#[allow(dead_code)]
struct BufferResource {
    buffer: Buffer,
    vertex: Option<BufferVertex>,
    vertex_count: Option<u32>,
    storage: Option<BufferStorage>,
}

struct ShaderResource {
    module: ShaderModule,
    entry: Option<String>,
    vertex_entry: Option<String>,
    fragment_entry: Option<String>,
}

enum PassResource {
    Compute {
        label: Option<String>,
        pipeline: ComputePipeline,
        bind_group: BindGroup,
        workgroups: [u32; 3],
    },
    Render {
        label: Option<String>,
        pipeline: RenderPipeline,
        bind_group: BindGroup,
        #[allow(dead_code)]
        clear: Option<RenderClear>,
        draw: Vec<RenderDraw>,
    },
    ShaderToy {
        label: Option<String>,
        pipeline: RenderPipeline,
        bind_group: BindGroup,
    },
}

#[allow(dead_code)]
struct CameraResource {
    camera: Camera,
    matrix: CameraMatrix,
}

#[allow(dead_code)]
enum UniformResource {
    Custom {
        content: Box<[u8]>,
        offsets: HashMap<String, usize>,
    },
    Internal,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ResourceError {
    InvalidShaderUtf8(Utf8Error),
    InvalidResource {
        id: String,
        reason: String,
    },
    IncorrectResource {
        id: String,
        expected: String,
        actual: String,
    },
    MissingResource {
        id: String,
    },
    MissingSetting {
        id: String,
    },
}

enum ShaderEntrypointType {
    COMPUTE,
    VERTEX,
    FRAGMENT,
}

pub struct Resources {
    buffers: HashMap<String, BufferResource>,
    cameras: HashMap<String, CameraResource>,
    uniforms: HashMap<String, UniformResource>,
    passes: Vec<PassResource>,
    setting_lookup: HashMap<String, String>,
    updated_uniforms: Vec<String>,
}

impl Resources {
    #[allow(dead_code)]
    pub fn new(
        scene: &Scene,
        device: &Device,
        width: u32,
        height: u32,
        format: TextureFormat,
    ) -> Result<Resources, ResourceError> {
        let descriptor = &scene.descriptor;

        let passes = Vec::<PassResource>::new();

        let mut buffers = HashMap::new();
        let mut cameras = HashMap::new();
        let mut uniforms = HashMap::new();

        let mut shaders: HashMap<String, ShaderResource> = HashMap::new();

        let mut setting_lookup: HashMap<String, String> = HashMap::new();

        // Construct builtin uniforms
        buffers.insert(
            "time".to_string(),
            BufferResource {
                buffer: device.create_buffer(&BufferDescriptor {
                    label: Some("Time Uniform"),
                    size: std::mem::size_of::<Time>() as u64,
                    mapped_at_creation: false,
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                }),
                vertex: None,
                vertex_count: None,
                storage: None,
            },
        );
        uniforms.insert("time".to_string(), UniformResource::Internal);

        buffers.insert(
            "shadertoy".to_string(),
            BufferResource {
                buffer: device.create_buffer(&BufferDescriptor {
                    label: Some("ShaderToy Uniform"),
                    size: std::mem::size_of::<ShaderToy>() as u64,
                    mapped_at_creation: false,
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                }),
                vertex: None,
                vertex_count: None,
                storage: None,
            },
        );
        uniforms.insert("shadertoy".to_string(), UniformResource::Internal);

        for (id, res) in descriptor.resources.iter() {
            match res {
                Resource::Buffer {
                    label,
                    size,
                    storage,
                    vertex,
                    vertices,
                } => {
                    let mut vertex = vertex.clone();
                    let mut usage = BufferUsages::empty();
                    if storage.is_some() {
                        usage |= BufferUsages::STORAGE;
                    }
                    if vertex.is_some() || vertices.is_some() {
                        usage |= BufferUsages::VERTEX;
                    }

                    let size = if let Some(size) = size {
                        *size
                    } else if let Some(vertices) = vertices {
                        if vertices.len() == 0 {
                            return Err(ResourceError::InvalidResource {
                                id: id.clone(),
                                reason: "Vertices must not be empty".to_string(),
                            });
                        }

                        let lengths: Vec<usize> =
                            vertices.iter().map(|vertex| vertex.len()).collect();
                        let length = lengths[0];
                        if !lengths.iter().all(|len| *len == length) {
                            return Err(ResourceError::InvalidResource {
                                id: id.clone(),
                                reason: "Vertices must all be the same size".to_string(),
                            });
                        }

                        vertex = Some(BufferVertex {
                            stride: length * mem::size_of::<f32>(),
                            step: Some(BufferVertexStep::Vertex),
                            attributes: vec![BufferVertexAttribute {
                                offset: 0,
                                location: 0,
                                format: BufferVertexAttributeFormat::Float32x2,
                            }],
                        });

                        vertices.len() * length * mem::size_of::<f32>()
                    } else {
                        return Err(ResourceError::InvalidResource {
                            id: id.clone(),
                            reason: "Buffer has neither size nor content".to_string(),
                        });
                    };

                    let buffer = if let Some(vertices) = vertices {
                        let mut contents = Vec::<u8>::new();

                        let length = vertices.len();
                        let size = vertices[0].len();

                        for i in 0..length {
                            for j in 0..size {
                                let bytes = bytemuck::bytes_of(&vertices[i][j]);

                                for k in 0..4 {
                                    contents.push(bytes[k]);
                                }
                            }
                        }

                        device.create_buffer_init(&BufferInitDescriptor {
                            label: label.clone().as_deref(),
                            contents: contents.as_slice(),
                            usage,
                        })
                    } else {
                        device.create_buffer(&BufferDescriptor {
                            label: label.clone().as_deref(),
                            size: size as BufferAddress,
                            usage,
                            mapped_at_creation: false,
                        })
                    };

                    buffers.insert(
                        id.clone(),
                        BufferResource {
                            buffer,
                            vertex: vertex.clone(),
                            vertex_count: vertex.as_ref().map(|v| (size / v.stride) as u32),
                            storage: storage.clone(),
                        },
                    );
                }
                Resource::Camera {
                    position, look_at, ..
                } => {
                    let eye =
                        Point3::<f32>::new(position[0] as _, position[1] as _, position[2] as _);
                    let target =
                        Point3::<f32>::new(look_at[0] as _, look_at[1] as _, look_at[2] as _);
                    let camera = Camera::new(eye, target, width, height);
                    let mut camera_matrix = CameraMatrix::new();
                    camera_matrix.update_view_proj(&camera);
                    let camera_buffer = device.create_buffer_init(&BufferInitDescriptor {
                        label: Some("Camera Buffer"),
                        contents: bytemuck::cast_slice(&[camera_matrix]),
                        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    });

                    cameras.insert(
                        id.clone(),
                        CameraResource {
                            camera,
                            matrix: camera_matrix,
                        },
                    );
                    buffers.insert(
                        id.clone(),
                        BufferResource {
                            buffer: camera_buffer,
                            vertex: None,
                            vertex_count: None,
                            storage: None,
                        },
                    );
                }
                Resource::Shader {
                    label,
                    main,
                    vertex_main,
                    fragment_main,
                    format,
                    stage,
                    ..
                } => {
                    let shader_source = scene
                        .files
                        .get(id)
                        .expect(format!("Shader source for {} was not loaded", id).as_str());

                    let shader_source_string = match std::str::from_utf8(shader_source.as_slice()) {
                        Ok(string) => string,
                        Err(error) => return Err(ResourceError::InvalidShaderUtf8(error)),
                    };

                    let module = match format.as_ref().unwrap_or(&ShaderFormat::Wgsl) {
                        ShaderFormat::Wgsl => device.create_shader_module(ShaderModuleDescriptor {
                            label: label.as_deref(),
                            source: wgpu::ShaderSource::Wgsl(Cow::Owned(
                                shader_source_string.to_string(),
                            )),
                        }),
                        ShaderFormat::Glsl => device.create_shader_module(ShaderModuleDescriptor {
                            label: label.as_deref(),
                            source: wgpu::ShaderSource::Glsl {
                                shader: Cow::Owned(shader_source_string.to_string()),
                                stage: stage
                                    .as_ref()
                                    .expect("GLSL shaders must specify a stage")
                                    .as_wgpu(),
                                defines: Default::default(),
                            },
                        }),
                        _ => todo!("unimplemented shader format {:?}", format.as_ref().unwrap()),
                    };

                    shaders.insert(
                        id.clone(),
                        ShaderResource {
                            module,
                            entry: main.clone(),
                            vertex_entry: vertex_main.clone(),
                            fragment_entry: fragment_main.clone(),
                        },
                    );
                }
                Resource::Uniform { label, values } => {
                    let mut content = Vec::<u8>::new();
                    let mut offsets = HashMap::<String, usize>::new();

                    let align_size = values
                        .iter()
                        .map(|value| {
                            if let Some(setting) = scene.settings.get(value) {
                                Ok(setting.alignment())
                            } else {
                                Err(ResourceError::MissingSetting { id: value.clone() })
                            }
                        })
                        .collect::<Result<Vec<usize>, ResourceError>>()?
                        .iter()
                        .max()
                        .map(|v| *v);

                    let align_size = if let Some(align_size) = align_size {
                        align_size
                    } else {
                        return Err(ResourceError::InvalidResource {
                            id: id.clone(),
                            reason: "Error finding the largest alignment of uniform values"
                                .to_string(),
                        });
                    };

                    for value in values {
                        setting_lookup.insert(value.clone(), id.clone());
                        if let Some(setting) = scene.settings.get(value) {
                            let index = content.len();

                            for _ in 0..align_size {
                                content.push(0);
                            }

                            setting.write(&mut content.as_mut_slice()[index..]);

                            offsets.insert(value.clone(), index);
                        } else {
                            return Err(ResourceError::MissingSetting { id: value.clone() });
                        }
                    }

                    let buffer = device.create_buffer_init(&BufferInitDescriptor {
                        label: label.clone().as_deref(),
                        contents: content.as_slice(),
                        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    });

                    buffers.insert(
                        id.clone(),
                        BufferResource {
                            buffer,
                            vertex: None,
                            vertex_count: None,
                            storage: None,
                        },
                    );

                    uniforms.insert(
                        id.clone(),
                        UniformResource::Custom {
                            content: content.into_boxed_slice(),
                            offsets,
                        },
                    );
                }
                Resource::ShaderToy { label, .. } => {
                    let shader_source = scene
                        .files
                        .get(id)
                        .expect(format!("Shader source for {} was not loaded", id).as_str());

                    let shader_source_string = match std::str::from_utf8(shader_source.as_slice()) {
                        Ok(string) => string,
                        Err(error) => return Err(ResourceError::InvalidShaderUtf8(error)),
                    };

                    let shader_harness = include_str!("../shaders/shadertoy/fragment.glsl");

                    let full_source_string =
                        shader_harness.replace("{{SOURCE}}", shader_source_string);

                    let module = device.create_shader_module(ShaderModuleDescriptor {
                        label: label.as_deref(),
                        source: wgpu::ShaderSource::Glsl {
                            shader: Cow::Owned(full_source_string),
                            stage: naga::ShaderStage::Fragment,
                            defines: Default::default(),
                        },
                    });
                    shaders.insert(
                        id.clone(),
                        ShaderResource {
                            module,
                            entry: None,
                            vertex_entry: None,
                            fragment_entry: Some("main".to_string()),
                        },
                    );
                    let module = device.create_shader_module(ShaderModuleDescriptor {
                        label: label.as_deref(),
                        source: wgpu::ShaderSource::Glsl {
                            shader: Cow::Owned(
                                include_str!("../shaders/shadertoy/vertex.glsl").to_string(),
                            ),
                            stage: naga::ShaderStage::Vertex,
                            defines: Default::default(),
                        },
                    });
                    shaders.insert(
                        "shadertoy_vertex_shader".to_string(),
                        ShaderResource {
                            module,
                            entry: None,
                            vertex_entry: Some("main".to_string()),
                            fragment_entry: None,
                        },
                    );
                }
            }
        }

        let mut resources = Resources {
            buffers,
            cameras,
            uniforms,
            passes,
            setting_lookup,
            updated_uniforms: Vec::new(),
        };

        for pass in descriptor.render_passes.iter() {
            let pass_resource = match pass {
                RenderPass::Compute { .. } => {
                    resources.build_compute_pipeline(pass, device, &shaders)?
                }
                RenderPass::Render { .. } => {
                    resources.build_render_pipeline(pass, device, format, &shaders)?
                }
                RenderPass::ShaderToy { .. } => {
                    resources.build_shadertoy_pipeline(pass, device, format, &shaders)?
                }
            };
            resources.passes.push(pass_resource);
        }

        Ok(resources)
    }

    fn get_shader_and_entrypoint<'a>(
        id: &String,
        entrypoint_type: ShaderEntrypointType,
        shaders: &'a HashMap<String, ShaderResource>,
    ) -> Result<(&'a ShaderModule, String), ResourceError> {
        /* why did i bother doing this?

                let shader_resource = scene
                    .descriptor
                    .resources
                    .get(&id)
                    .expect("Shader went missing");

                Resources::validate_resource(shader_resource, ResourceType::Shader, id)?;
        */
        let shader = match shaders.get(id) {
            Some(shader) => shader,
            None => panic!("Shader {} was not initialized", id),
        };

        let entrypoint = match entrypoint_type {
            ShaderEntrypointType::COMPUTE => {
                if let Some(entrypoint) = shader.entry.as_ref() {
                    entrypoint
                } else {
                    return Err(ResourceError::InvalidResource {
                        id: id.clone(),
                        reason: "Shader does not have a compute entrypoint".to_string(),
                    });
                }
            }
            ShaderEntrypointType::VERTEX => {
                if let Some(entrypoint) = shader.vertex_entry.as_ref() {
                    entrypoint
                } else {
                    return Err(ResourceError::InvalidResource {
                        id: id.clone(),
                        reason: "Shader does not have a vertex entrypoint".to_string(),
                    });
                }
            }
            ShaderEntrypointType::FRAGMENT => {
                if let Some(entrypoint) = shader.fragment_entry.as_ref() {
                    entrypoint
                } else {
                    return Err(ResourceError::InvalidResource {
                        id: id.clone(),
                        reason: "Shader does not have a fragment entrypoint".to_string(),
                    });
                }
            }
        };

        Ok((&shader.module, entrypoint.clone()))
    }

    fn build_bind_group(
        &self,
        label: &Option<String>,
        bindings: Option<&Vec<String>>,
        bindings_visibility: Option<&Vec<RenderPipelineBindingVisibility>>,
        device: &Device,
    ) -> Result<(BindGroupLayout, BindGroup), ResourceError> {
        let mut bind_group_layout_entries = Vec::<BindGroupLayoutEntry>::new();

        if let Some(bindings) = bindings {
            let compute_vis = bindings
                .iter()
                .map(|_| RenderPipelineBindingVisibility::Compute)
                .collect();

            let visibilities = if let Some(vis) = bindings_visibility {
                vis
            } else {
                &compute_vis
            };

            for (idx, binding) in bindings.iter().enumerate() {
                let bind_type = if self.cameras.contains_key(binding) {
                    BufferBindingType::Uniform
                } else if self.uniforms.contains_key(binding) {
                    BufferBindingType::Uniform
                } else if let Some(buf) = self.buffers.get(binding) {
                    if let Some(storage) = buf.storage.as_ref() {
                        BufferBindingType::Storage {
                            read_only: storage.storage_type == BufferStorageType::Read,
                        }
                    } else {
                        return Err(ResourceError::InvalidResource {
                            id: binding.clone(),
                            reason: "Attempted to bind buffer, but it is not a storage buffer"
                                .to_string(),
                        });
                    }
                } else {
                    return Err(ResourceError::IncorrectResource {
                        id: binding.clone(),
                        expected: "Bindable resource".to_string(),
                        actual: "Not bindable".to_string(),
                    });
                };

                bind_group_layout_entries.push(BindGroupLayoutEntry {
                    binding: idx as u32,
                    visibility: visibilities[idx].as_wgpu(),
                    ty: BindingType::Buffer {
                        ty: bind_type,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                });
            }
        }

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: label
                .clone()
                .map(|s| format!("{} (Bind Group Layout)", s))
                .as_deref(),
            entries: bind_group_layout_entries.as_slice(),
        });

        let mut bind_group_entries = Vec::<BindGroupEntry>::new();

        if let Some(bindings) = bindings {
            for (idx, binding) in bindings.iter().enumerate() {
                let buffer = if let Some(buffer) = self.buffers.get(binding) {
                    buffer
                } else {
                    panic!("Binding {} missing buffer", binding);
                };
                bind_group_entries.push(BindGroupEntry {
                    binding: idx as u32,
                    resource: buffer.buffer.as_entire_binding(),
                });
            }
        }

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: bind_group_entries.as_slice(),
        });

        Ok((bind_group_layout, bind_group))
    }

    fn build_compute_pipeline(
        &self,
        pass: &RenderPass,
        device: &Device,
        shaders: &HashMap<String, ShaderResource>,
    ) -> Result<PassResource, ResourceError> {
        let (label, pipeline, workgroups) = match pass {
            RenderPass::Compute {
                label,
                pipeline,
                workgroups,
            } => (label, pipeline, workgroups),
            _ => panic!("how did we get here"),
        };

        let (bind_group_layout, bind_group) =
            self.build_bind_group(label, Some(&pipeline.bindings), None, device)?;

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: label
                .clone()
                .map(|s| format!("{} (Pipeline Layout)", s))
                .as_deref(),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let (shader, entry_point) = Resources::get_shader_and_entrypoint(
            &pipeline.shader,
            ShaderEntrypointType::COMPUTE,
            &shaders,
        )?;

        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: label.as_deref(),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: entry_point.deref(),
        });

        Ok(PassResource::Compute {
            label: label.clone(),
            pipeline: compute_pipeline,
            bind_group,
            workgroups: [workgroups[0], workgroups[1], workgroups[2]],
        })
    }

    fn build_render_pipeline(
        &mut self,
        pass: &RenderPass,
        device: &Device,
        format: TextureFormat,
        shaders: &HashMap<String, ShaderResource>,
    ) -> Result<PassResource, ResourceError> {
        let (label, pipeline, clear, draw) = match pass {
            RenderPass::Render {
                label,
                pipeline,
                clear,
                draw,
            } => (label, pipeline, clear, draw),
            _ => panic!("how did we get here"),
        };

        if pipeline.bindings.as_ref().map(|vec| vec.len())
            != pipeline.bindings_visibility.as_ref().map(|vec| vec.len())
        {
            return Err(ResourceError::InvalidResource {
                id: "Render Pipeline".to_string(),
                reason: "Bindings and Bindings Visibility do not have matching lengths".to_string(),
            });
        }

        let (bind_group_layout, bind_group) = self.build_bind_group(
            label,
            pipeline.bindings.as_ref(),
            pipeline.bindings_visibility.as_ref(),
            device,
        )?;

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: label
                .clone()
                .map(|s| format!("{} (Pipeline Layout)", s))
                .as_deref(),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let (shader, vertex_entry) = Resources::get_shader_and_entrypoint(
            &pipeline.shader_vertex,
            ShaderEntrypointType::VERTEX,
            &shaders,
        )?;

        let fragment_module_and_entry = if let Some(shader) = pipeline.shader_fragment.as_ref() {
            let (module, entry_point) = Resources::get_shader_and_entrypoint(
                shader,
                ShaderEntrypointType::FRAGMENT,
                &shaders,
            )?;

            Some((module, entry_point))
        } else {
            None
        };

        let targets = [Some(ColorTargetState {
            format,
            blend: Some(BlendState::REPLACE),
            write_mask: ColorWrites::ALL,
        })];

        let mut attributes = Vec::<VertexAttribute>::new();
        let mut buffers = Vec::<VertexBufferLayout>::new();

        if let Some(vertex) = pipeline.vertex.as_ref() {
            attributes.extend(vertex.attributes().iter());
            buffers.push(vertex.desc(attributes.as_slice()));
        }

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: label.as_deref(),
            layout: Some(&pipeline_layout),

            vertex: VertexState {
                module: &shader,
                entry_point: vertex_entry.deref(),
                buffers: buffers.as_slice(),
            },
            fragment: if fragment_module_and_entry.is_some() {
                let (module, entry) = fragment_module_and_entry.as_ref().unwrap();

                Some(FragmentState {
                    module: module,
                    entry_point: entry.as_str(),
                    targets: &targets,
                })
            } else {
                None
            },
            primitive: PrimitiveState {
                topology: pipeline.topology.as_wgpu(),
                strip_index_format: None,
                front_face: pipeline.front_face.as_wgpu(),
                cull_mode: pipeline.cull_mode.as_wgpu(),
                polygon_mode: pipeline.polygon_mode.as_wgpu(),
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // TODO: ensure our Draw params a) match vertex layout and b) are valid resources

        Ok(PassResource::Render {
            label: label.clone(),
            pipeline: render_pipeline,
            bind_group,
            clear: clear.clone(),
            draw: draw.clone(),
        })
    }

    fn build_shadertoy_pipeline(
        &mut self,
        pass: &RenderPass,
        device: &Device,
        format: TextureFormat,
        shaders: &HashMap<String, ShaderResource>,
    ) -> Result<PassResource, ResourceError> {
        let (label, source, additional_bindings) = match pass {
            RenderPass::ShaderToy {
                label,
                source,
                bindings,
            } => (label, source, bindings),
            _ => panic!("how did we get here"),
        };

        let mut bindings = vec!["shadertoy".to_string()];
        if let Some(additional_bindings) = additional_bindings {
            for binding in additional_bindings {
                bindings.push(binding.to_owned());
            }
        }

        let bindings_visibility = bindings
            .iter()
            .map(|_| RenderPipelineBindingVisibility::Fragment)
            .collect();

        let (bind_group_layout, bind_group) =
            self.build_bind_group(label, Some(&bindings), Some(&bindings_visibility), device)?;

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: label
                .clone()
                .map(|s| format!("{} (Pipeline Layout)", s))
                .as_deref(),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let (shader, vertex_entry) = Resources::get_shader_and_entrypoint(
            &"shadertoy_vertex_shader".to_string(),
            ShaderEntrypointType::VERTEX,
            &shaders,
        )?;

        let (fragment_module, fragment_entry_point) =
            Resources::get_shader_and_entrypoint(source, ShaderEntrypointType::FRAGMENT, &shaders)?;

        let targets = [Some(ColorTargetState {
            format,
            blend: Some(BlendState::REPLACE),
            write_mask: ColorWrites::ALL,
        })];

        let quad_buffer = BufferResource {
            buffer: device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Quad Buffer"),
                contents: bytemuck::cast_slice(VERTICES_QUAD),
                usage: BufferUsages::VERTEX,
            }),
            vertex: Some(BufferVertex {
                stride: 16,
                step: Some(BufferVertexStep::Vertex),
                attributes: vec![
                    BufferVertexAttribute {
                        offset: 0,
                        location: 0,
                        format: BufferVertexAttributeFormat::Float32x2,
                    },
                    BufferVertexAttribute {
                        offset: 8,
                        location: 1,
                        format: BufferVertexAttributeFormat::Float32x2,
                    },
                ],
            }),
            vertex_count: Some(6),
            storage: None,
        };

        self.buffers
            .insert("shadertoy_quad".to_string(), quad_buffer);

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: label.as_deref(),
            layout: Some(&pipeline_layout),

            vertex: VertexState {
                module: &shader,
                entry_point: vertex_entry.deref(),
                buffers: &[VertexBufferLayout {
                    array_stride: 16,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        VertexAttribute {
                            offset: 8,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                    ],
                }],
            },
            fragment: Some(FragmentState {
                module: fragment_module,
                entry_point: fragment_entry_point.as_str(),
                targets: &targets,
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Ok(PassResource::ShaderToy {
            label: label.clone(),
            pipeline: render_pipeline,
            bind_group,
        })
    }

    pub fn update_setting(&mut self, key: String, value: Setting) {
        if let Some(uniform_id) = self.setting_lookup.get(&key) {
            // update data in uniform
            let (content, offsets) = match self.uniforms.get_mut(uniform_id) {
                Some(uniform) => match uniform {
                    UniformResource::Custom { content, offsets } => (content, offsets),
                    UniformResource::Internal => {
                        panic!("Tried to update internal uniform {}", uniform_id)
                    }
                },
                None => panic!(
                    "Setting was linked to uniform {} with no buffer.",
                    uniform_id
                ),
            };

            let index = match offsets.get(&key) {
                Some(index) => *index,
                None => panic!("Uniform {} setting {} missing index", uniform_id, key),
            };
            let end = index + value.size();
            value.write(&mut content.as_mut()[index..end]);

            self.updated_uniforms.push(uniform_id.clone());
        }
    }

    pub fn render(
        &mut self,
        queue: &Queue,
        view: &TextureView,
        encoder: &mut CommandEncoder,
        time: Time,
        shadertoy: ShaderToy,
    ) {
        if let Some(time_buffer) = self.buffers.get(&"time".to_string()) {
            queue.write_buffer(&time_buffer.buffer, 0, bytemuck::cast_slice(&[time]));
        }
        if let Some(shadertoy_buffer) = self.buffers.get(&"shadertoy".to_string()) {
            queue.write_buffer(
                &shadertoy_buffer.buffer,
                0,
                bytemuck::cast_slice(&[shadertoy]),
            );
        }

        for uniform_id in self.updated_uniforms.drain(..) {
            let content = match self.uniforms.get(&uniform_id) {
                Some(uniform) => match uniform {
                    UniformResource::Custom { content, .. } => content,
                    UniformResource::Internal => {
                        panic!("Tried to update internal uniform {}", uniform_id)
                    }
                },
                None => panic!("Updated uniform {} that doesn't exist.", uniform_id),
            };

            let buffer = match self.buffers.get(&uniform_id) {
                Some(buffer) => buffer,
                None => panic!("Updated uniform {} with no buffer.", uniform_id),
            };

            queue.write_buffer(&buffer.buffer, 0, content.as_ref());
        }

        for pass in self.passes.iter() {
            match pass {
                PassResource::Compute {
                    label,
                    pipeline,
                    bind_group,
                    workgroups,
                } => {
                    if let Some(label) = label {
                        encoder.push_debug_group(label);
                    }
                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Vertex Compute Pass"),
                    });

                    cpass.set_pipeline(pipeline);
                    cpass.set_bind_group(0, bind_group, &[]);
                    cpass.dispatch_workgroups(workgroups[0], workgroups[1], workgroups[2]);

                    drop(cpass);
                    if let Some(_) = label {
                        encoder.pop_debug_group();
                    }
                }
                PassResource::Render {
                    label,
                    pipeline,
                    bind_group,
                    clear: _,
                    draw,
                } => {
                    if let Some(label) = label {
                        encoder.push_debug_group(label);
                    }

                    // todo: load from clear
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Scene Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });

                    rpass.set_pipeline(pipeline);
                    for draw in draw {
                        rpass.set_bind_group(0, bind_group, &[]);
                        let mut vertices = draw.vertex_count.unwrap_or(0);
                        let instances = draw.instances.unwrap_or(1);

                        if let Some(vertex_buffer) = draw.vertex_buffer.as_ref() {
                            let vertex_buffer = self.buffers.get(vertex_buffer).unwrap();
                            rpass.set_vertex_buffer(0, vertex_buffer.buffer.slice(..));
                            vertices = vertex_buffer
                                .vertex_count
                                .expect("Vertex buffer has no vertex count");
                        }

                        rpass.draw(0..vertices, 0..instances);
                    }

                    drop(rpass);

                    if let Some(_) = label {
                        encoder.pop_debug_group();
                    }
                }
                PassResource::ShaderToy {
                    label,
                    pipeline,
                    bind_group,
                } => {
                    if let Some(label) = label {
                        encoder.push_debug_group(label);
                    }

                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Scene Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });

                    rpass.set_pipeline(pipeline);
                    rpass.set_bind_group(0, bind_group, &[]);

                    let vertex_buffer = self.buffers.get("shadertoy_quad").unwrap();
                    rpass.set_vertex_buffer(0, vertex_buffer.buffer.slice(..));
                    let vertices = vertex_buffer
                        .vertex_count
                        .expect("Quad buffer has no vertex count");
                    rpass.draw(0..vertices, 0..1);

                    drop(rpass);

                    if let Some(_) = label {
                        encoder.pop_debug_group();
                    }
                }
            }
        }
    }
}
