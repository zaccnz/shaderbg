/*
 * Main window
 */
use imgui::*;
use imgui_wgpu::{Renderer, RendererConfig};
use pollster::block_on;
use std::time::Instant;
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent as TaoWindowEvent},
    event_loop::{ControlFlow, EventLoopWindowTarget},
    keyboard::KeyCode,
    window::{Window as TaoWindow, WindowBuilder, WindowId},
};
use wgpu::{Device, Queue, Surface};

use crate::{
    app::{AppEvent, AppState, WindowEvent},
    ext::{self, imgui_tao_support::TaoPlatform},
};

pub struct Window {
    window: TaoWindow,
    surface: Surface,
    imgui: Context,
    renderer: Renderer,
    device: Device,
    platform: TaoPlatform,
    queue: Queue,
    app_state: AppState,

    last_frame: Instant,
    demo_open: bool,
    last_cursor: Option<MouseCursor>,
}

impl Window {
    pub fn build(event_loop: &EventLoopWindowTarget<WindowEvent>, app_state: AppState) -> Window {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        #[cfg(target_os = "macos")]
        {
            use tao::platform::macos::{ActivationPolicy, EventLoopWindowTargetExtMacOS};
            event_loop.set_activation_policy_at_runtime(ActivationPolicy::Regular);
        }

        let window = WindowBuilder::new()
            .with_title("shaderbg")
            .with_inner_size(LogicalSize::new(1024, 576))
            .build(&event_loop)
            .unwrap();

        #[cfg(target_os = "macos")]
        {
            window.set_focus();
        }

        let surface = unsafe { instance.create_surface(&window) }.unwrap();

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

        // Set up dear imgui
        let mut imgui = imgui::Context::create();
        let mut platform = TaoPlatform::init(&mut imgui);
        platform.attach_window(
            imgui.io_mut(),
            &window,
            ext::imgui_tao_support::HiDpiMode::Default,
        );
        imgui.set_ini_filename(None);

        let font_size = (13.0 * hidpi_factor) as f32;
        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        imgui.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);

        let renderer_config = RendererConfig {
            texture_format: surface_desc.format,
            ..Default::default()
        };

        let renderer = Renderer::new(&mut imgui, &device, &queue, renderer_config);

        Window {
            window,
            surface,
            imgui,
            renderer,
            device,
            platform,
            queue,
            app_state,
            last_frame: Instant::now(),
            demo_open: false,
            last_cursor: None,
        }
    }

    pub fn get_window_id(&self) -> WindowId {
        self.window.id()
    }

    pub fn handle(&mut self, event: Event<WindowEvent>, _control_flow: &mut ControlFlow) -> bool {
        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };

        match event {
            Event::WindowEvent {
                event: TaoWindowEvent::CloseRequested,
                ..
            } => {
                return false;
            }
            Event::WindowEvent {
                event: TaoWindowEvent::Resized(_),
                ..
            } => {
                let size = self.window.inner_size();

                let surface_desc = wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    width: size.width,
                    height: size.height,
                    present_mode: wgpu::PresentMode::Fifo,
                    alpha_mode: wgpu::CompositeAlphaMode::Auto,
                    view_formats: vec![wgpu::TextureFormat::Bgra8Unorm],
                };

                self.surface.configure(&self.device, &surface_desc);
            }
            Event::WindowEvent {
                event:
                    TaoWindowEvent::KeyboardInput {
                        event:
                            tao::event::KeyEvent {
                                physical_key: KeyCode::Escape,
                                ..
                            },
                        ..
                    },
                ..
            } => {
                return false;
            }
            Event::MainEventsCleared => self.window.request_redraw(),
            Event::RedrawEventsCleared => {
                let now = Instant::now();
                self.imgui.io_mut().update_delta_time(now - self.last_frame);
                self.last_frame = now;

                let frame = match self.surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(e) => {
                        eprintln!("dropped frame: {e:?}");
                        return true;
                    }
                };
                self.platform
                    .prepare_frame(self.imgui.io_mut(), &self.window)
                    .expect("Failed to prepare frame");
                let ui = self.imgui.frame();

                {
                    let window = ui.window("Hello world");
                    window
                        .size([300.0, 200.0], Condition::FirstUseEver)
                        .build(|| {
                            ui.text("imgui-rs on WGPU & Tao!");
                            ui.separator();
                            let mut tray_on = self.app_state.get_state().tray_open;
                            ui.checkbox("Tray", &mut tray_on);
                            if tray_on != self.app_state.get_state().tray_open {
                                let event = if tray_on {
                                    WindowEvent::StartTray
                                } else {
                                    WindowEvent::CloseTray
                                };
                                self.app_state.send_event(AppEvent::Window(event)).unwrap();
                            }
                            if ui.button("Show ImGui Demo Window") {
                                self.demo_open = true;
                            }
                        });

                    if self.demo_open {
                        ui.show_demo_window(&mut self.demo_open);
                    }
                }

                let mut encoder: wgpu::CommandEncoder = self
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                if self.last_cursor != ui.mouse_cursor() {
                    self.last_cursor = ui.mouse_cursor();
                    self.platform.prepare_render(ui, &self.window);
                }

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

                self.renderer
                    .render(self.imgui.render(), &self.queue, &self.device, &mut rpass)
                    .expect("Rendering failed");

                drop(rpass);

                self.queue.submit(Some(encoder.finish()));

                frame.present();
            }
            _ => (),
        }

        self.platform
            .handle_event(self.imgui.io_mut(), &self.window, &event);

        true
    }

    pub fn will_close(&self, event_loop: &EventLoopWindowTarget<WindowEvent>) {
        #[cfg(target_os = "macos")]
        {
            use tao::platform::macos::{ActivationPolicy, EventLoopWindowTargetExtMacOS};
            event_loop.set_activation_policy_at_runtime(ActivationPolicy::Accessory);
        }
    }
}
