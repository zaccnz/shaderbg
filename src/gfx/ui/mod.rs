use std::time::Instant;

use imgui::{Condition, Context, FontSource, MouseCursor};
use imgui_wgpu::{Renderer, RendererConfig};
use tao::{event::Event, window::Window};
use wgpu::{CommandEncoder, Device, Queue, TextureFormat, TextureView};

use crate::{
    app::{AppEvent, AppState, WindowEvent},
    ext::{self, imgui_tao_support::TaoPlatform},
};

pub struct Ui {
    imgui: Context,
    renderer: Renderer,
    platform: TaoPlatform,
    app_state: AppState,

    last_frame: Instant,
    demo_open: bool,
    last_cursor: Option<MouseCursor>,
}

impl Ui {
    pub fn new(
        window: &Window,
        device: &Device,
        queue: &Queue,
        hidpi_factor: f64,
        texture_format: TextureFormat,
        app_state: AppState,
    ) -> Ui {
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
            texture_format,
            ..Default::default()
        };

        let renderer = Renderer::new(&mut imgui, &device, &queue, renderer_config);

        Ui {
            imgui,
            renderer,
            platform,
            app_state,
            last_frame: Instant::now(),
            demo_open: false,
            last_cursor: None,
        }
    }

    pub fn render<'a>(
        &'a mut self,
        window: &Window,
        queue: &Queue,
        device: &Device,
        view: &TextureView,
        encoder: &mut CommandEncoder,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("UI Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        let now = Instant::now();
        self.imgui.io_mut().update_delta_time(now - self.last_frame);
        self.last_frame = now;

        self.platform
            .prepare_frame(self.imgui.io_mut(), window)
            .expect("Failed to prepare frame");
        let ui = self.imgui.frame();

        {
            let window = ui.window("Hello world");
            window
                .position([800.0, 75.0], Condition::FirstUseEver)
                .size([200.0, 110.0], Condition::FirstUseEver)
                .build(|| {
                    ui.text("imgui-rs on WGPU & Tao!");
                    ui.separator();
                    let mut tray_on = self.app_state.get().tray_open;
                    ui.checkbox("Tray", &mut tray_on);
                    if tray_on != self.app_state.get().tray_open {
                        let event = if tray_on {
                            WindowEvent::StartTray
                        } else {
                            WindowEvent::CloseTray
                        };
                        self.app_state.send(AppEvent::Window(event)).unwrap();
                    }
                    if ui.button("Show ImGui Demo Window") {
                        self.demo_open = true;
                    }
                });

            /*
            if let Some(scene) = scene {
                let window = ui.window("Scene Settings");
                window
                    .size([300.0, 400.0], Condition::FirstUseEver)
                    .build(|| {
                        ui.text("WIP");
                        //scene.ui(ui);
                    });
            }*/

            if self.demo_open {
                ui.show_demo_window(&mut self.demo_open);
            }
        }

        if self.last_cursor != ui.mouse_cursor() {
            self.last_cursor = ui.mouse_cursor();
            self.platform.prepare_render(ui, window);
        }

        self.renderer
            .render(self.imgui.render(), queue, device, &mut render_pass)
            .expect("Rendering failed");
    }

    pub fn handle_event(&mut self, window: &Window, event: &Event<crate::app::WindowEvent>) {
        self.platform
            .handle_event(self.imgui.io_mut(), &window, &event);
    }
}
