use std::collections::HashMap;
use wgpu::{
    BindGroupLayout, CommandEncoder, Device, Queue, RenderPipeline, Sampler, Texture,
    TextureDescriptor, TextureFormat, TextureView, TextureViewDescriptor,
};

use crate::{
    gfx::buffer::{ShaderToy, Time},
    scene::{io::Metadata, Resources, Scene},
};

const PREVIEW_WIDTH: u32 = 128;
const PREVIEW_HEIGHT: u32 = 72;
const PREVIEW_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;
const PREVIEW_POST_PROCESS_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

pub struct Browser {
    scenes: Box<[(String, Metadata)]>,
    previews: HashMap<String, ScenePreview>,
    preview_post_process: ScenePreviewPostProcess,
    preview_shadertoy: ShaderToy,
}

impl Browser {
    pub fn new(scenes: Vec<(String, &Scene)>, device: &Device) -> Browser {
        let mut previews = HashMap::new();

        for (name, scene) in scenes.iter() {
            previews.insert(name.clone(), ScenePreview::new(&scene, &device, &name));
        }

        let preview_post_process = ScenePreviewPostProcess::new(device);

        Browser {
            scenes: scenes
                .iter()
                .map(|(name, scene)| (name.clone(), scene.descriptor.meta.clone()))
                .collect(),
            previews,
            preview_post_process,
            preview_shadertoy: ShaderToy::new(),
        }
    }

    pub fn update_previews(
        &mut self,
        renderer: &mut egui_wgpu::Renderer,
        queue: &Queue,
        device: &mut Device,
        time: Time,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Preview Encoder"),
        });
        self.preview_shadertoy
            .update(time.time, time.dt as f64, PREVIEW_WIDTH, PREVIEW_HEIGHT);
        for (_, preview) in self.previews.iter_mut() {
            preview.render(
                renderer,
                queue,
                device,
                &mut encoder,
                time,
                &self.preview_post_process,
                self.preview_shadertoy,
            );
        }
        queue.submit(Some(encoder.finish()));
    }

    pub fn render(
        &self,
        ui: &mut egui::Ui,
        current_scene: Option<usize>,
        reload: Option<&mut bool>,
    ) -> Option<usize> {
        let mut selected = None;
        ui.horizontal(|ui| {
            ui.heading(format!("{} scenes loaded", self.scenes.len()));
            if let Some(reload) = reload {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Reload").clicked() {
                        *reload = true;
                    }
                });
            }
        });
        ui.separator();
        for (index, (name, meta)) in self.scenes.iter().enumerate() {
            let preview = self.previews.get(name);

            ui.horizontal(|ui| {
                if let Some(preview) = preview {
                    if let Some(texture) = preview.egui_texture.as_ref() {
                        ui.image(
                            *texture,
                            egui::Vec2::new(PREVIEW_WIDTH as f32, PREVIEW_HEIGHT as f32),
                        );
                    }
                }

                ui.vertical(|ui| {
                    ui.label(format!("{} ({})", meta.name, meta.version));
                    if Some(index) == current_scene {
                        ui.label("selected");
                    }
                    if ui.button("Select").clicked() {
                        selected = Some(index)
                    }
                })
            });
        }
        selected
    }
}

struct ScenePreview {
    resources: Resources,
    texture: Texture,
    texture_out: Texture,
    egui_texture: Option<egui::epaint::TextureId>,
}

impl ScenePreview {
    pub fn new(scene: &Scene, device: &Device, name: &String) -> ScenePreview {
        ScenePreview {
            resources: Resources::new(scene, device, PREVIEW_WIDTH, PREVIEW_HEIGHT, PREVIEW_FORMAT)
                .unwrap(),
            texture: device.create_texture(&TextureDescriptor {
                label: Some(format!("Scene Preview {}", name).as_str()),
                size: wgpu::Extent3d {
                    width: PREVIEW_WIDTH,
                    height: PREVIEW_HEIGHT,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: PREVIEW_FORMAT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[PREVIEW_FORMAT],
            }),
            texture_out: device.create_texture(&TextureDescriptor {
                label: Some(format!("Scene Preview {}", name).as_str()),
                size: wgpu::Extent3d {
                    width: PREVIEW_WIDTH,
                    height: PREVIEW_HEIGHT,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: PREVIEW_POST_PROCESS_FORMAT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[PREVIEW_POST_PROCESS_FORMAT],
            }),
            egui_texture: None,
        }
    }

    pub fn render(
        &mut self,
        renderer: &mut egui_wgpu::Renderer,
        queue: &Queue,
        device: &mut Device,
        encoder: &mut CommandEncoder,
        time: Time,
        post_process: &ScenePreviewPostProcess,
        shadertoy: ShaderToy,
    ) {
        let view = self.texture.create_view(&TextureViewDescriptor {
            label: Some("Scene Preview Texture View"),
            format: Some(PREVIEW_FORMAT),
            dimension: Some(wgpu::TextureViewDimension::D2),
            ..Default::default()
        });
        self.resources
            .render(queue, &view, encoder, time, shadertoy);

        let view_out = self.texture_out.create_view(&TextureViewDescriptor {
            label: Some("Scene Preview Post Process Texture View"),
            format: Some(PREVIEW_POST_PROCESS_FORMAT),
            dimension: Some(wgpu::TextureViewDimension::D2),
            ..Default::default()
        });
        post_process.apply(&view, &view_out, encoder, device);
        if let Some(texture) = self.egui_texture.as_ref() {
            renderer.update_egui_texture_from_wgpu_texture(
                device,
                &view_out,
                wgpu::FilterMode::Linear,
                *texture,
            );
        } else {
            self.egui_texture =
                Some(renderer.register_native_texture(device, &view_out, wgpu::FilterMode::Linear));
        }
    }
}

struct ScenePreviewPostProcess {
    pipeline: RenderPipeline,
    texture_sampler: Sampler,
    bind_group_layout: BindGroupLayout,
}

impl ScenePreviewPostProcess {
    pub fn new(device: &Device) -> ScenePreviewPostProcess {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Scene Preview Post Process Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/postprocess.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("Scene Preview Post Process Bind Group Layout"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Scene Preview Post Process Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Scene Preview Post Process Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "postprocess_to_srgb",
                targets: &[Some(wgpu::ColorTargetState {
                    format: PREVIEW_POST_PROCESS_FORMAT,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back),
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

        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        ScenePreviewPostProcess {
            pipeline,
            texture_sampler,
            bind_group_layout,
        }
    }

    pub fn apply(
        &self,
        view: &TextureView,
        view_out: &TextureView,
        encoder: &mut CommandEncoder,
        device: &mut Device,
    ) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.texture_sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Post Process Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: view_out,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }
    }
}
