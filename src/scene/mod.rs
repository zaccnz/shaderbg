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

use rand::Rng;
use wgpu::{
    include_wgsl,
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, Buffer, BufferUsages, CommandEncoder, Device, Queue, RenderPipeline,
    SurfaceConfiguration, TextureView,
};

use crate::gfx::{
    buffer::{CameraMatrix, Index, Vertex},
    camera::Camera,
};

// TMP: constants for the wave scene
const WIDTH: u32 = 100;
const HEIGHT: u32 = 80;
const WC: usize = (WIDTH + 1) as _;
const HC: usize = (HEIGHT + 1) as _;
const WAVE_NOISE: f32 = 4.0;
const NIL_VERTEX: Vertex = Vertex {
    position: [0.0, 0.0, 0.0],
};

pub struct Scene {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    num_indices: u32,
    render_pipeline: RenderPipeline,
    camera: Camera,
    camera_buffer: Buffer,
    camera_matrix: CameraMatrix,
    camera_bind_group: BindGroup,
}

impl Scene {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Scene {
        let shader = device.create_shader_module(include_wgsl!("waves.wgsl"));

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

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
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

        let mut vertices: [Vertex; WC * HC] = [NIL_VERTEX; WC * HC];
        let mut gg: [[Index; HC]; WC] = [[0; HC]; WC];

        let mut rng = rand::thread_rng();

        // build vertices
        let mut idx = 0 as usize;
        for i in 0..=WIDTH {
            for j in 0..=HEIGHT {
                let vertex = Vertex {
                    position: [
                        (i as f32 - (WIDTH as f32 * 0.5)) * 18.0,
                        rng.gen_range(0.0..=WAVE_NOISE) - 10.0,
                        ((HEIGHT as f32 * 0.5) - j as f32) * 18.0,
                    ],
                };
                vertices[(i * (HEIGHT + 1) + j) as usize] = vertex;
                gg[i as usize][j as usize] = idx as Index;
                idx += 1;
            }
        }

        let pe = |num: &mut usize| {
            let tmp = *num;
            *num += 1;
            tmp
        };

        // build indices
        let mut indices: [Index; WC * HC * 6] = [0; WC * HC * 6];
        let mut idx = 0 as usize;
        for i in 1..=WIDTH {
            for j in 1..=HEIGHT {
                let d = gg[i as usize][j as usize];
                let b = gg[i as usize][(j - 1) as usize];
                let c = gg[(i - 1) as usize][j as usize];
                let a = gg[(i - 1) as usize][(j - 1) as usize];

                if rng.gen_bool(0.5) {
                    indices[pe(&mut idx)] = a;
                    indices[pe(&mut idx)] = b;
                    indices[pe(&mut idx)] = c;
                    indices[pe(&mut idx)] = b;
                    indices[pe(&mut idx)] = c;
                    indices[pe(&mut idx)] = d;
                } else {
                    indices[pe(&mut idx)] = a;
                    indices[pe(&mut idx)] = b;
                    indices[pe(&mut idx)] = d;
                    indices[pe(&mut idx)] = a;
                    indices[pe(&mut idx)] = c;
                    indices[pe(&mut idx)] = d;
                }
            }
        }

        // build buffers
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("index buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: BufferUsages::INDEX,
        });

        Scene {
            vertex_buffer,
            index_buffer,
            render_pipeline,
            camera,
            camera_buffer,
            camera_matrix,
            camera_bind_group,
            num_indices: idx as u32,
        }
    }

    pub fn _update(&mut self, _delta: f64) {
        todo!("Update uniforms");
    }

    pub fn render<'a>(
        &'a mut self,
        queue: &Queue,
        view: &TextureView,
        encoder: &mut CommandEncoder,
    ) {
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

        // NOTE: because time is part of the camera matrix (laziness), I must update the camera every frame
        self.camera_matrix.update_view_proj(&self.camera);
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_matrix]),
        );

        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_bind_group(0, &self.camera_bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        rpass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
}
