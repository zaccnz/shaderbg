/*
 * scene module.  stores all code related to scene loading, preferences and rendering
 * right now, i am hardcoding the waves scene
 * https://github.com/tengbao/vanta/blob/master/src/vanta.waves.js
 *
 * in the future, this will be constructed from TOML files which describe
 *   - settings which are user changable
 *   - shader files to be included
 *   - uniforms for said shaders
 *   - ShaderToy compat uniforms
 *   - scene metadata
 *
 * i would like for all vanta scenes to become backgrounds, as this would
 * make for a good example of how to use the program.  i will also port
 * some shadertoys to work as well.
 */

use imgui::Ui;
use wgpu::{
    include_wgsl,
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, Buffer, BufferDescriptor, CommandEncoder, ComputePipeline, Device, Queue,
    RenderPipeline, SurfaceConfiguration, TextureView,
};

use crate::{
    app::AppState,
    gfx::{
        buffer::{CameraMatrix, Vertex},
        camera::Camera,
    },
};

pub struct Scene {
    state: AppState,
    compute_pipeline: ComputePipeline,
    wave_params_buffer: Buffer,
    wave_render_params_buffer: Buffer,
    compute_bind_group: BindGroup,
    render_pipeline: RenderPipeline,
    //camera: Camera,
    //camera_buffer: Buffer,
    //camera_matrix: CameraMatrix,
    render_bind_group: BindGroup,
    vertex_buffer: Buffer,
    //size: BufferAddress,
    last_colour: [f32; 3],
}

const VERTEX_COUNT: u32 = 100 * 80 * 6;

impl Scene {
    pub fn new(state: AppState, device: &Device, config: &SurfaceConfiguration) -> Scene {
        let shader_compute = device.create_shader_module(include_wgsl!("vertex.wgsl"));

        // COMPUTE SHADER INIT
        let params = state.get().scene_params;

        let wave_params_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Wave Param Buffer"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let slice_size = (VERTEX_COUNT * 3) as usize * std::mem::size_of::<f32>();
        let size = slice_size as wgpu::BufferAddress;

        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Vertex Buffer"),
            size,
            mapped_at_creation: false,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
        });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("compute_bind_group_layout"),
            });
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wave_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: vertex_buffer.as_entire_binding(),
                },
            ],
            label: Some("compute_bind_group"),
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader_compute,
            entry_point: "main",
        });

        // RENDER SHADER INIT
        let shader_render = device.create_shader_module(include_wgsl!("waves.wgsl"));

        let wave_render_params_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Wave Render Param Buffer"),
            contents: bytemuck::cast_slice(&[state.get().scene_render_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera = Camera::new(
            (240.0, 200.0, 390.0).into(),
            (140.0, -30.0, 190.0).into(),
            config.width,
            config.height,
        );

        let mut camera_matrix = CameraMatrix::new();
        camera_matrix.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_matrix]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("render_bind_group_layout"),
            });

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wave_render_params_buffer.as_entire_binding(),
                },
            ],
            label: Some("render_bind_group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&render_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_render,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_render,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
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

        Scene {
            state,
            compute_pipeline,
            wave_params_buffer,
            wave_render_params_buffer,
            compute_bind_group,
            render_pipeline,
            //camera,
            //camera_buffer,
            //camera_matrix,
            render_bind_group,
            vertex_buffer,
            //size,
            last_colour: [0.0, 0.0, 0.0],
        }
    }

    pub fn render<'a>(
        &'a mut self,
        queue: &Queue,
        view: &TextureView,
        encoder: &mut CommandEncoder,
    ) {
        encoder.push_debug_group("Scene Compute");
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Vertex Compute Pass"),
        });

        let params = self.state.get().scene_params;
        queue.write_buffer(&self.wave_params_buffer, 0, bytemuck::cast_slice(&[params]));

        cpass.set_pipeline(&self.compute_pipeline);
        cpass.set_bind_group(0, &self.compute_bind_group, &[]);
        cpass.dispatch_workgroups(100, 80, 1);

        drop(cpass);
        encoder.pop_debug_group();

        encoder.push_debug_group("Scene Render");
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

        queue.write_buffer(
            &self.wave_render_params_buffer,
            0,
            bytemuck::cast_slice(&[self.state.get().scene_render_params]),
        );

        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_bind_group(0, &self.render_bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.draw(0..VERTEX_COUNT, 0..1);

        drop(rpass);
        encoder.pop_debug_group();
    }

    pub fn ui(&mut self, ui: &Ui) {
        let mut change = false;
        let mut wp = self.state.get().scene_params;
        change |= ui.slider("Size", 1.0, 30.0, &mut wp.size);
        change |= ui.slider("Speed", 0.0, 4.0, &mut wp.speed);
        change |= ui.slider("Height", 1.0, 20.0, &mut wp.height);
        change |= ui.slider("Noise", 0.0, 10.0, &mut wp.noise);

        if change {
            self.state
                .send(crate::app::AppEvent::UpdateSceneParams(wp))
                .unwrap();
        }

        let mut change = false;

        let mut wrp = self.state.get().scene_render_params;
        let imgui_colour = [wrp.colour[0], wrp.colour[1], wrp.colour[2], 1.0];

        let mut open = ui.color_button("Colour", imgui_colour);
        ui.same_line_with_spacing(0.0, unsafe { ui.style() }.item_inner_spacing[0]);
        open |= ui.button("Pick colour");
        if open {
            ui.open_popup("picker");
            self.last_colour = wrp.colour;
        }
        if let Some(popup) = ui.begin_popup("picker") {
            change |= ui.color_picker3("Wave Colour", &mut wrp.colour);

            if ui.button("Save") {
                ui.close_current_popup();
            }
            ui.same_line_with_spacing(0.0, unsafe { ui.style() }.item_inner_spacing[0]);
            if ui.button("Cancel") {
                ui.close_current_popup();
                wrp.colour = self.last_colour;
                change = true;
            }
            popup.end();
        }

        if change {
            self.state
                .send(crate::app::AppEvent::UpdateSceneRenderParams(wrp))
                .unwrap();
        }
    }
}
