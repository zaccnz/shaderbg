use std::{collections::HashMap, rc::Rc};
use web_time::{Instant, SystemTime};

use log::info;
use shaderbg_render::{
    gfx::{buffer::Time, Gfx, GfxContext},
    scene::{Resources, Scene},
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

async fn run() {
    info!("Starting web build");

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("shaderbg web")
        .build(&event_loop)
        .unwrap();

    let window = Rc::new(window);

    #[cfg(target_family = "wasm")]
    {
        wasm::insert_canvas(window.as_ref());
        wasm::hook_resize(window.clone());
    }

    let gfx_context = GfxContext::new(window.as_ref());

    let size = window.inner_size();
    let mut gfx = Gfx::new(gfx_context, size.width, size.height).await;

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

    let scene = match Scene::load_from_memory(scene_toml, scene_files) {
        Ok(scene) => scene,
        Err(e) => panic!("{:?}", e),
    };

    let mut time = Time::new();

    let mut resources = Resources::new(&scene, &gfx.device, &gfx.config, time).unwrap();
    let mut last_frame = Instant::now();
    let started = SystemTime::now();

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            window_id,
        } if window_id == window.id() => *control_flow = ControlFlow::Exit,
        Event::WindowEvent {
            event: WindowEvent::Resized(PhysicalSize { width, height }),
            ..
        } => {
            info!("resized");
            gfx.resized(width, height);
        }
        Event::WindowEvent {
            event:
                WindowEvent::ScaleFactorChanged {
                    new_inner_size: PhysicalSize { width, height },
                    ..
                },
            ..
        } => {
            gfx.resized(*width, *height);
        }
        Event::MainEventsCleared => window.request_redraw(),
        Event::RedrawRequested(_) => {
            let now = Instant::now();
            let dt = (now - last_frame).as_secs_f64();
            let now_u32 = SystemTime::now()
                .duration_since(started)
                .unwrap()
                .as_millis() as u32;
            time.update_time(now_u32, dt);
            last_frame = now;

            gfx.render(Some(&mut resources), time);
        }
        _ => (),
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
    use winit::{dpi::LogicalSize, event::Event, window::Window};

    const CANVAS_ID: &str = "winit-canvas";

    pub fn insert_canvas(window: &Window) {
        use winit::platform::web::WindowExtWebSys;

        let canvas = window.canvas();

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let body = document.body().unwrap();

        body.style().set_property("margin", "0px");

        canvas.set_id(CANVAS_ID);
        canvas
            .style()
            .set_property("background-color", "crimson")
            .unwrap();
        body.append_child(&canvas).unwrap();
    }

    pub fn hook_resize(window: std::rc::Rc<Window>) {
        let html_window = web_sys::window().unwrap();

        {
            let window = window.clone();
            let resized = Closure::<dyn FnMut()>::new(move || {
                let html_window = web_sys::window().unwrap();

                let canvas: HtmlCanvasElement = html_window
                    .document()
                    .unwrap()
                    .get_element_by_id(CANVAS_ID)
                    .expect("Failed to find render canvas")
                    .dyn_into()
                    .expect(
                        format!("Canvas '{}' was not an HtmlCanvasElement!", CANVAS_ID).as_str(),
                    );

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

            html_window
                .add_event_listener_with_callback("load", resized.as_ref().unchecked_ref())
                .unwrap();

            resized.forget();
        }
    }
}
