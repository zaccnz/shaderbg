/*
 * Main window
 */
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent as TaoWindowEvent},
    event_loop::{ControlFlow, EventLoopWindowTarget},
    keyboard::KeyCode,
    window::{Window as TaoWindow, WindowBuilder, WindowId},
};

use crate::{
    app::{AppState, WindowEvent},
    gfx::{ui::Ui, Gfx, GfxContext},
};

pub struct Window {
    window: TaoWindow,
    gfx: Gfx,
    #[allow(dead_code)]
    app_state: AppState,
    ui: Ui,
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

        let gfx = Gfx::new(gfx_context, &window);

        let ui = Ui::new(
            &window,
            &gfx.device,
            &gfx.queue,
            gfx.hidpi_factor,
            gfx.config.format,
            app_state.clone(),
        );

        Window {
            window,
            gfx,
            app_state,
            ui,
        }
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
                event: TaoWindowEvent::Resized(_) | TaoWindowEvent::ScaleFactorChanged { .. },
                ..
            } => {
                self.gfx.resized(&self.window);
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
                self.gfx.render(&self.window, Some(&mut self.ui));
            }
            _ => (),
        }

        self.gfx
            .handle_event(&self.window, &event, Some(&mut self.ui));
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
