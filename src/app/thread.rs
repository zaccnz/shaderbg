/*
 * Window thread.  Runs the Event Loop and handles all WindowEvent messages
 */
use tao::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
};

use crate::app::{AppEvent, AppState, Tray, Window};

#[derive(Debug)]
pub enum WindowEvent {
    OpenWindow,
    StartTray,
    CloseTray,
}

pub struct WindowThread {
    was_window_open: bool,
    window: Option<Window>,
    tray: Option<Tray>,
}

impl WindowThread {
    pub fn build() -> (WindowThread, EventLoop<WindowEvent>) {
        let mut event_loop = EventLoop::<WindowEvent>::with_user_event();
        /*
               let tray = Some(Tray::build(&event_loop));
               let window = Some(Window::build(&event_loop));
        */

        #[cfg(target_os = "macos")]
        {
            use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
            event_loop.set_activation_policy(ActivationPolicy::Accessory);
        }

        (
            WindowThread {
                was_window_open: false,
                window: None,
                tray: None,
            },
            event_loop,
        )
    }

    fn handle_window_event(
        &mut self,
        event: Event<WindowEvent>,
        event_loop: &EventLoopWindowTarget<WindowEvent>,
        control_flow: &mut ControlFlow,
    ) {
        let window = self.window.take();

        if let Some(mut window) = window {
            if window.handle(event, control_flow) {
                self.window = Some(window);
            } else {
                window.will_close(event_loop);
            }
        }
    }

    fn handle_tray_event(
        &mut self,
        event: Event<WindowEvent>,
        event_loop: &EventLoopWindowTarget<WindowEvent>,
        control_flow: &mut ControlFlow,
    ) {
        let tray = self.tray.take();

        if let Some(mut tray) = tray {
            tray.handle(event, event_loop, control_flow);
            self.tray = Some(tray);
        }
    }

    pub fn run(mut self, event_loop: EventLoop<WindowEvent>, app_state: AppState) {
        event_loop.run(move |event, event_loop, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::NewEvents(StartCause::Init) => {
                    println!("Event::NewEvents(StartCause::Init)");

                    let state = app_state.get_state();
                    if state.window_open {
                        self.window = Some(Window::build(event_loop, app_state.clone()));
                    }
                    if state.tray_open {
                        self.tray = Some(Tray::build(event_loop, app_state.clone()));
                    }
                    app_state.send_event(AppEvent::EventLoopReady).unwrap();

                    /*
                    let background = WindowBuilder::new()
                        .with_title("shaderbg background")
                        .build(event_loop)
                        .unwrap();

                    std::thread::spawn(move || {
                        background.set_title("shaderbg background (in other thread)");

                        loop {}
                    });
                    */
                }
                Event::LoopDestroyed => {
                    println!("Loop Destroyed");
                    self.tray.take();
                }
                Event::UserEvent(window_event) => {
                    match window_event {
                        WindowEvent::OpenWindow => {
                            if self.window.is_some() {
                                panic!("Cannot open window - already open");
                            }
                            self.window = Some(Window::build(event_loop, app_state.clone()));
                        }
                        WindowEvent::StartTray => {
                            if self.tray.is_some() {
                                panic!("Cannot start tray - already started");
                            }
                            self.tray = Some(Tray::build(event_loop, app_state.clone()));
                            app_state
                                .send_event(AppEvent::TrayStateChange(true))
                                .unwrap();
                        }
                        WindowEvent::CloseTray => {
                            self.tray.take();
                            app_state
                                .send_event(AppEvent::TrayStateChange(false))
                                .unwrap();
                        }
                    }
                    println!("{:?}", window_event);
                }
                Event::WindowEvent { window_id, .. } => {
                    if let Some(win) = &self.window {
                        if window_id == win.get_window_id() {
                            self.handle_window_event(event, event_loop, control_flow);
                        }
                    } else {
                        // check if window_id == background_window_id
                        // forward event to background proc.
                    }
                }
                Event::RedrawEventsCleared | Event::MainEventsCleared => {
                    // todo: forward event to background proc.
                    self.handle_window_event(event, event_loop, control_flow);
                }
                Event::MenuEvent { .. } | Event::TrayEvent { .. } => {
                    self.handle_tray_event(event, event_loop, control_flow)
                }
                _ => (),
            }

            #[cfg(target_os = "macos")]
            if self.window.is_some() != self.was_window_open {
                use tao::platform::macos::{ActivationPolicy, EventLoopWindowTargetExtMacOS};
                event_loop.set_activation_policy_at_runtime(if self.window.is_some() {
                    ActivationPolicy::Regular
                } else {
                    ActivationPolicy::Accessory
                });
            }
            self.was_window_open = self.window.is_some();

            if self.window.is_none() && self.tray.is_none() {
                *control_flow = ControlFlow::Exit;
            }
        });
    }
}
