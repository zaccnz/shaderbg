/*
 * Main window
 */
use tao::{
    dpi::{LogicalSize, PhysicalSize},
    event::{Event, WindowEvent as TaoWindowEvent},
    event_loop::{ControlFlow, EventLoopWindowTarget},
    keyboard::KeyCode,
    window::{Window as TaoWindow, WindowBuilder, WindowId},
};

use crate::app::{AppState, WindowEvent};
use shaderbg_render::{
    gfx::{Gfx, GfxContext},
    scene::{Resources, Setting},
};

pub struct Window {
    window: TaoWindow,
    gfx: Gfx,
    #[allow(dead_code)]
    app_state: AppState,
    // ui: Ui,
    resources: Resources,
}

impl Window {
    pub fn build(event_loop: &EventLoopWindowTarget<WindowEvent>, app_state: AppState) -> Window {
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

        let gfx_context = GfxContext::new(&window);

        let size = window.inner_size();
        let gfx = pollster::block_on(Gfx::new(gfx_context, size.width, size.height, false));

        // let scene = Scene::new(app_state.clone(), &gfx.device, &gfx.config);
        /*
        let ui = Ui::new(
            &window,
            &gfx.device,
            &gfx.queue,
            gfx.hidpi_factor,
            gfx.config.format,
            app_state.clone(),
        );
        */

        let resources = Resources::new(
            &app_state.get().scene,
            &gfx.device,
            &gfx.config,
            app_state.get().time,
        )
        .unwrap();

        Window {
            window,
            gfx,
            app_state,
            resources,
        }
    }

    pub fn update_setting(&mut self, key: String, value: Setting) {
        self.resources.update_setting(key, value);
    }

    pub fn get_window_id(&self) -> WindowId {
        self.window.id()
    }

    pub fn handle(&mut self, event: Event<WindowEvent>, _control_flow: &mut ControlFlow) -> bool {
        match event {
            Event::WindowEvent {
                event: TaoWindowEvent::CloseRequested,
                ..
            } => {
                return false;
            }
            Event::WindowEvent {
                event: TaoWindowEvent::Resized(PhysicalSize { width, height }),
                ..
            } => {
                self.gfx.resized(width, height);
            }
            Event::WindowEvent {
                event:
                    TaoWindowEvent::ScaleFactorChanged {
                        new_inner_size: PhysicalSize { width, height },
                        ..
                    },
                ..
            } => {
                self.gfx.resized(*width, *height);
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
                self.gfx.render(
                    Some(&mut self.resources),
                    self.app_state.get().time,
                    None,
                    |_| {},
                );
            }
            _ => (),
        }

        //self.gfx
        //    .handle_event(&self.window, &event, Some(&mut self.ui));
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
