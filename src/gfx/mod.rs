use pollster::block_on;
use tao::{dpi::PhysicalSize, event::Event, window::Window};
use wgpu::{Device, Instance, Queue, Surface, SurfaceConfiguration};

use crate::{gfx::ui::Ui, scene::Resources};

pub mod buffer;
pub mod camera;
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

        let (device, queue) = block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::DeviceDescriptor::default().features
                    | wgpu::Features::POLYGON_MODE_LINE,
                ..Default::default()
            },
            None,
        ))
        .unwrap();

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

        Gfx {
            config: surface_desc,
            surface,
            device,
            queue,
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

    pub fn render(&mut self, window: &Window, scene: Option<&mut Resources>, ui: Option<&mut Ui>) {
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

        let mut encoder: wgpu::CommandEncoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        if let Some(scene) = scene {
            scene.render(&self.queue, &view, &mut encoder);
        }

        if let Some(ui) = ui {
            ui.render(window, &self.queue, &self.device, &view, &mut encoder);
        }

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
