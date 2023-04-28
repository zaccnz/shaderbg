use pollster::block_on;
use tao::{dpi::PhysicalSize, event::Event, window::Window};
use wgpu::{include_wgsl, Device, Instance, Queue, RenderPipeline, Surface, SurfaceConfiguration};

use crate::gfx::ui::Ui;

pub mod ui;

// because we cannot create a surface on second thread,
// we create a context on the main thread which is used
// to construct Gfx on another thread
pub struct GfxContext {
    pub instance: Instance,
    pub surface: Surface,
}

impl GfxContext {
    pub fn new(window: &Window) -> GfxContext {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // TODO: windows - use HWND of background process
        // https://stackoverflow.com/questions/56132584/draw-on-windows-10-wallpaper-in-c
        // (should work, as long as we can make a HasRawWindowHandle + HasRawDisplayHandle object!)
        // https://docs.rs/wgpu/latest/wgpu/struct.Instance.html#method.create_surface
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        GfxContext { instance, surface }
    }
}

pub struct Gfx {
    pub config: SurfaceConfiguration,
    surface: Surface,
    pub device: Device,
    pub queue: Queue,
    render_pipeline: RenderPipeline,
    size: PhysicalSize<u32>,
    pub hidpi_factor: f64,
}

impl Gfx {
    pub fn new(context: GfxContext, window: &Window) -> Gfx {
        let instance = context.instance;
        let surface = context.surface;

        let hidpi_factor = window.scale_factor();

        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None)).unwrap();

        let size = window.inner_size();

        let surface_desc = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![wgpu::TextureFormat::Bgra8Unorm],
        };

        surface.configure(&device, &surface_desc);

        let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_desc.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
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

        Gfx {
            config: surface_desc,
            surface,
            device,
            queue,
            render_pipeline,
            size,
            hidpi_factor,
        }
    }

    pub fn resized(&mut self, window: &Window) {
        self.size = window.inner_size();

        self.config.width = self.size.width;
        self.config.height = self.size.height;

        self.surface.configure(&self.device, &self.config);
    }

    pub fn render(&mut self, window: &Window, ui: Option<&mut Ui>) {
        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };

        let mut encoder: wgpu::CommandEncoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                eprintln!("dropped frame: {e:?}");
                return;
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        rpass.set_pipeline(&self.render_pipeline);
        rpass.draw(0..3, 0..1);

        if let Some(ui) = ui {
            ui.render(window, &self.queue, &self.device, &mut rpass);
        }

        drop(rpass);

        self.queue.submit(Some(encoder.finish()));

        frame.present();
    }

    pub fn handle_event(
        &mut self,
        window: &Window,
        event: &Event<crate::app::WindowEvent>,
        ui: Option<&mut Ui>,
    ) {
        if let Some(ui) = ui {
            ui.handle_event(window, event);
        }
    }
}
