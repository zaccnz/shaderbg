use wgpu::{Device, Instance, Queue, Surface, SurfaceConfiguration};

use crate::scene::Resources;

use self::buffer::{ShaderToy, Time};

pub mod buffer;
pub mod camera;
pub mod ui;
pub mod vertices;
pub use ui::Ui;

// because we cannot create a surface on second thread,
// we create a context on the main thread which is used
// to construct Gfx on another thread
pub struct GfxContext {
    pub instance: Instance,
    pub surface: Surface,
}

impl GfxContext {
    pub fn new<W>(window: &W) -> GfxContext
    where
        W: raw_window_handle::HasRawWindowHandle + raw_window_handle::HasRawDisplayHandle,
    {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
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
    pub ui: Option<Ui>,
}

impl Gfx {
    pub async fn new(context: GfxContext, width: u32, height: u32, with_ui: bool) -> Gfx {
        let instance = context.instance;
        let surface = context.surface;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    /*features: wgpu::DeviceDescriptor::default().features
                    | wgpu::Features::POLYGON_MODE_LINE,*/
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();

        let surface_desc = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width: width,
            height: height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![wgpu::TextureFormat::Bgra8Unorm],
        };

        surface.configure(&device, &surface_desc);

        let mut ui = None;
        if with_ui {
            ui = Some(Ui::new(&device, surface_desc.format));
        }

        Gfx {
            config: surface_desc,
            surface,
            device,
            queue,
            ui,
        }
    }

    pub fn resized(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;

        self.surface.configure(&self.device, &self.config);
    }

    pub fn render<F: FnOnce(&egui::Context)>(
        &mut self,
        scene: Option<&mut Resources>,
        time: Time,
        shadertoy: ShaderToy,
        ui_input: Option<(f32, egui::RawInput)>, // f32 -> pixels_per_point
        ui_render: F,
    ) -> Option<egui::FullOutput> {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                eprintln!("dropped frame: {e:?}");
                return None;
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder: wgpu::CommandEncoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        if let Some(scene) = scene {
            scene.render(&self.queue, &view, &mut encoder, time, shadertoy);
        }

        let mut full_output = None;

        if let Some(ui) = self.ui.as_mut() {
            let (pixels_per_point, input) =
                ui_input.expect("Gfx::render() with scene, expected Some(ui_input) got None");
            let output = ui.render(
                &mut encoder,
                &self.device,
                &self.queue,
                &view,
                ui_render,
                input,
                pixels_per_point,
                self.config.width,
                self.config.height,
            );

            full_output = Some(output);
        }

        self.queue.submit(Some(encoder.finish()));

        frame.present();

        full_output
    }
}
