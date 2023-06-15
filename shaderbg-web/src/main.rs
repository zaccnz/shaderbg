use egui::RichText;
use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
};
use web_time::{Instant, SystemTime};

use shaderbg_render::{
    gfx::{
        self,
        buffer::{ShaderToy, Time},
        Gfx, GfxContext,
    },
    scene::{Resources, Scene},
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};

fn demo_ui(
    ui: &mut egui::Ui,
    settings_open: &mut bool,
    scene_ui: &mut gfx::ui::Scene,
    scene: &Scene,
    fps_average: f64,
) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(scene.descriptor.meta.name.clone())
                .heading()
                .strong(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Settings").clicked() {
                scene_ui.load_settings(scene);
                *settings_open = true;
            };
        });
    });
    egui::Grid::new("metadata_grid").show(ui, |ui| {
        ui.label(RichText::new("Version:").strong());
        ui.label(scene.descriptor.meta.version.clone());
        ui.end_row();

        ui.label(RichText::new("Description:").strong());
        ui.label(scene.descriptor.meta.description.clone());
        ui.end_row();

        ui.label(RichText::new("Author:").strong());
        ui.label(scene.descriptor.meta.author.clone());
        ui.end_row();
    });
    ui.separator();
    ui.heading("How does it work?");
    ui.label("The waves scene is described in three files");

    egui::Grid::new("detail_grid").show(ui, |ui| {
        ui.hyperlink_to(
            "scene.toml",
            "https://github.com/zaccnz/shaderbg/blob/main/scenes/waves/scene.toml",
        );
        ui.label("describes the scene");
        ui.end_row();

        ui.hyperlink_to(
            "vertices.wgsl",
            "https://github.com/zaccnz/shaderbg/blob/main/scenes/waves/vertices.wgsl",
        );
        ui.label("compute shader that generates wave vertices");
        ui.end_row();

        ui.hyperlink_to(
            "waves.wgsl",
            "https://github.com/zaccnz/shaderbg/blob/main/scenes/waves/waves.wgsl",
        );
        ui.label("render shader (vertex and fragment) that places and colours the waves");
        ui.end_row();
    });

    ui.label("It is then rendered using WGPU-RS.  This runs the shaders natively (using Vulkan, Metal, DirectX 11/12 - or WebGPU in this demo).");
    ui.label("There is no webview or game engine.");
    ui.separator();
    ui.vertical_centered(|ui| {
        ui.hyperlink_to("Get the full version", "https://github.com/zaccnz/shaderbg");
        ui.label("(still in development)");
        ui.label(format!("{:.0} fps", fps_average));
    });
}

#[derive(Debug)]
pub enum ThemeEvent {
    Dark,
    Light,
}

async fn run() {
    let event_loop = EventLoopBuilder::<ThemeEvent>::with_user_event().build();
    #[cfg(target_family = "wasm")]
    let proxy = event_loop.create_proxy();
    let window = WindowBuilder::new()
        .with_title("shaderbg web")
        .build(&event_loop)
        .unwrap();

    let window = Rc::new(window);

    #[cfg(target_family = "wasm")]
    wasm::insert_canvas(window.as_ref());

    let gfx_context = GfxContext::new(window.as_ref());

    let size = window.inner_size();
    let mut gfx = Gfx::new(gfx_context, size.width, size.height, true).await;

    #[cfg(target_family = "wasm")]
    {
        let visuals = wasm::hook_resize_and_color_scheme(window.clone(), proxy.clone());

        gfx.ui.as_ref().unwrap().context().set_visuals(visuals);
    }

    let mut egui_platform = egui_winit::State::new(window.as_ref());
    egui_platform.set_pixels_per_point(window.scale_factor() as f32);

    // hack to avoid WASM file operations
    let scene_toml = include_bytes!("../../scenes/waves/scene.toml").to_vec();
    let scene_files = HashMap::from([
        (
            "compute_shader".to_string(),
            include_bytes!("../../scenes/waves/vertices.wgsl").to_vec(),
        ),
        (
            "render_shader".to_string(),
            include_bytes!("../../scenes/waves/waves.wgsl").to_vec(),
        ),
    ]);

    let mut scene = match Scene::load_from_memory(scene_toml, scene_files) {
        Ok(scene) => scene,
        Err(e) => panic!("{:?}", e),
    };

    let mut time = Time::new();
    let mut shadertoy = ShaderToy::new();

    let mut resources = Resources::new(&scene, &gfx.device, &gfx.config, time, shadertoy).unwrap();
    let mut last_frame = Instant::now();
    let started = SystemTime::now();

    let mut settings_open = false;

    let mut scene_ui = gfx::ui::Scene::new(&scene.descriptor);

    let mut frame_times = VecDeque::new();

    event_loop.run(move |event, _, control_flow| {
        if let Event::WindowEvent { event, .. } = &event {
            if let Some(ui) = gfx.ui.as_ref() {
                let _ = egui_platform.on_event(&ui.context, event);
            }
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            Event::WindowEvent {
                event: WindowEvent::Resized(PhysicalSize { width, height }),
                ..
            } => {
                gfx.resized(width, height);
            }
            Event::WindowEvent {
                event:
                    WindowEvent::ScaleFactorChanged {
                        new_inner_size: PhysicalSize { width, height },
                        scale_factor,
                    },
                ..
            } => {
                gfx.resized(*width, *height);
                egui_platform.set_pixels_per_point(scale_factor as f32);
            }
            Event::UserEvent(ThemeEvent::Dark) => {
                if let Some(ui) = gfx.ui.as_ref() {
                    ui.context().set_visuals(egui::Visuals::dark())
                }
            }
            Event::UserEvent(ThemeEvent::Light) => {
                if let Some(ui) = gfx.ui.as_ref() {
                    ui.context().set_visuals(egui::Visuals::light())
                }
            }
            Event::MainEventsCleared => window.request_redraw(),
            Event::RedrawRequested(_) => {
                let now = Instant::now();
                let dt = (now - last_frame).as_secs_f64();
                let now_u32 = SystemTime::now()
                    .duration_since(started)
                    .unwrap()
                    .as_millis() as u32;
                frame_times.push_back(dt);
                if frame_times.len() > 100 {
                    frame_times.pop_front().unwrap();
                }
                let total_frame_time: f64 = frame_times.iter().sum();
                let fps_average = frame_times.len() as f64 / total_frame_time;
                time.update_time(now_u32, dt);
                let size = window.inner_size();
                shadertoy.update(now_u32, dt, size.width, size.height);
                last_frame = now;

                let mut changes = Vec::new();

                let full_output = gfx.render(
                    Some(&mut resources),
                    time,
                    shadertoy,
                    Some((
                        egui_platform.pixels_per_point(),
                        egui_platform.take_egui_input(&window),
                    )),
                    |ctx| {
                        egui::Window::new("shaderbg web demo")
                            .movable(false)
                            .resizable(false)
                            .show(ctx, |ui| {
                                demo_ui(ui, &mut settings_open, &mut scene_ui, &scene, fps_average);
                            });

                        let mut open = settings_open;

                        egui::Window::new("Scene Settings")
                            .open(&mut open)
                            .resizable(false)
                            .show(ctx, |ui| {
                                settings_open =
                                    scene_ui.render(ui, scene.settings.clone(), &mut changes);
                            });

                        settings_open &= open;
                    },
                );

                for (key, value) in changes {
                    scene.settings.update(&key, value.clone());
                    resources.update_setting(key, value);
                }

                if let Some(full_output) = full_output {
                    if let Some(ui) = gfx.ui.as_ref() {
                        egui_platform.handle_platform_output(
                            &window,
                            &ui.context,
                            full_output.platform_output.clone(),
                        );
                    }
                }
            }
            _ => (),
        }
    });
}

#[cfg(not(target_family = "wasm"))]
pub fn main() {
    pollster::block_on(run())
}

#[cfg(target_family = "wasm")]
pub fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Debug).expect("error initializing logger");

    wasm_bindgen_futures::spawn_local(run());
}

#[cfg(target_family = "wasm")]
mod wasm {
    use wasm_bindgen::prelude::*;
    use web_sys::HtmlCanvasElement;
    use winit::{dpi::LogicalSize, event_loop::EventLoopProxy, window::Window};

    use crate::ThemeEvent;

    const CANVAS_ID: &str = "winit-canvas";

    pub fn insert_canvas(winit_window: &Window) {
        use winit::platform::web::WindowExtWebSys;

        let canvas = winit_window.canvas();

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let body = document.body().unwrap();

        body.style().set_property("margin", "0px").unwrap();

        canvas.set_id(CANVAS_ID);
        canvas
            .style()
            .set_property("background-color", "crimson")
            .unwrap();

        let width = window
            .inner_width()
            .expect("Failed to read window width")
            .as_f64()
            .unwrap() as u32;

        let height = window
            .inner_height()
            .expect("Failed to read window height")
            .as_f64()
            .unwrap() as u32;

        canvas.set_width(width);
        canvas.set_height(height);
        winit_window.set_inner_size(LogicalSize { width, height });

        body.append_child(&canvas).unwrap();
    }

    pub fn hook_resize_and_color_scheme(
        window: std::rc::Rc<Window>,
        update_visuals: EventLoopProxy<ThemeEvent>,
    ) -> egui::Visuals {
        let html_window = web_sys::window().unwrap();

        let window = window.clone();
        let resized = Closure::<dyn FnMut()>::new(move || {
            let html_window = web_sys::window().unwrap();

            let canvas: HtmlCanvasElement = html_window
                .document()
                .unwrap()
                .get_element_by_id(CANVAS_ID)
                .expect("Failed to find render canvas")
                .dyn_into()
                .expect(format!("Canvas '{}' was not an HtmlCanvasElement!", CANVAS_ID).as_str());

            let width = html_window
                .inner_width()
                .expect("Failed to read window width")
                .as_f64()
                .unwrap() as u32;

            let height = html_window
                .inner_height()
                .expect("Failed to read window height")
                .as_f64()
                .unwrap() as u32;

            canvas.set_width(width);
            canvas.set_height(height);
            window.set_inner_size(LogicalSize { width, height });
        });

        html_window
            .add_event_listener_with_callback("resize", resized.as_ref().unchecked_ref())
            .unwrap();

        resized.forget();

        let dark_query = html_window.match_media("(prefers-color-scheme: dark)");

        if let Some(media_query_list) = dark_query.unwrap() {
            let colour_theme_changes =
                Closure::<dyn FnMut(_)>::new(move |event: web_sys::MediaQueryListEvent| {
                    update_visuals
                        .send_event(if event.matches() {
                            ThemeEvent::Dark
                        } else {
                            ThemeEvent::Light
                        })
                        .unwrap();
                });
            media_query_list
                .add_event_listener_with_callback(
                    "change",
                    colour_theme_changes.as_ref().unchecked_ref(),
                )
                .unwrap();

            colour_theme_changes.forget();

            if media_query_list.matches() {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            }
        } else {
            egui::Visuals::dark()
        }
    }
}
