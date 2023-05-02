use std::time::Instant;

/*
 * Window thread.  Runs the Event Loop and handles all WindowEvent messages
 */
use tao::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
};

use crate::app::{AppEvent, AppEventSender, AppState, Background, Tray, Window};

#[derive(Debug)]
pub enum WindowEvent {
    OpenWindow,
    StartTray,
    CloseTray,
    CreateBackgroundWindow,
    CloseBackgroundWindow,
}

enum WindowEventTarget {
    None,
    Window,
    Background,
}

pub struct WindowThread {
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

    pub fn run(
        mut self,
        event_loop: EventLoop<WindowEvent>,
        app_state: AppState,
        handle: std::thread::JoinHandle<()>,
    ) {
        let mut handle = Some(handle);
        let mut background_window_id = None;
        let mut last_frame = Instant::now();

        event_loop.run(move |event, event_loop, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::NewEvents(StartCause::Init) => {
                    println!("Event::NewEvents(StartCause::Init)");

                    {
                        let state = app_state.get();
                        if state.window_open {
                            self.window = Some(Window::build(event_loop, app_state.clone()));
                        }
                        if state.tray_open {
                            self.tray = Some(Tray::build(event_loop, app_state.clone()));
                        }
                    }
                    app_state.send(AppEvent::EventLoopReady).unwrap();

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
                    app_state.send(AppEvent::EventLoopQuit).unwrap();
                    handle.take().unwrap().join().unwrap();
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
                            app_state.send(AppEvent::TrayStateChange(true)).unwrap();
                        }
                        WindowEvent::CloseTray => {
                            self.tray.take();
                            app_state.send(AppEvent::TrayStateChange(false)).unwrap();
                        }
                        WindowEvent::CreateBackgroundWindow => {
                            let background = Background::new(
                                event_loop,
                                app_state.clone_for(AppEventSender::Background),
                            );

                            background_window_id = Some(background.window.id());

                            app_state
                                .send(AppEvent::BackgroundCreated(background))
                                .unwrap();
                        }
                        WindowEvent::CloseBackgroundWindow => {
                            background_window_id.take();
                        }
                    }
                    println!("{:?}", window_event);
                }
                Event::WindowEvent { window_id, .. } => {
                    let mut target = WindowEventTarget::None;

                    if let Some(win) = &self.window {
                        if window_id == win.get_window_id() {
                            target = WindowEventTarget::Window;
                        }
                    }

                    if let Some(id) = background_window_id {
                        if id == window_id {
                            target = WindowEventTarget::Background;
                        }
                    }

                    match target {
                        WindowEventTarget::Window => {
                            self.handle_window_event(event, event_loop, control_flow);
                        }
                        WindowEventTarget::Background => {
                            if let Some(event) = event.to_static() {
                                app_state.send(AppEvent::BackgroundEvent(event)).unwrap();
                            }
                        }
                        _ => {
                            eprintln!("Window event has no target (window_id={:?})", window_id);
                        }
                    }
                }
                Event::RedrawEventsCleared => {
                    self.handle_window_event(event, event_loop, control_flow);
                    if background_window_id.is_some() {
                        app_state
                            .send(AppEvent::BackgroundEvent(Event::RedrawEventsCleared))
                            .unwrap();
                    }
                }
                Event::MainEventsCleared => {
                    let now = Instant::now();
                    app_state
                        .send(AppEvent::Update((now - last_frame).as_secs_f64()))
                        .unwrap();
                    last_frame = now;

                    self.handle_window_event(event, event_loop, control_flow);
                    if background_window_id.is_some() {
                        app_state
                            .send(AppEvent::BackgroundEvent(Event::MainEventsCleared))
                            .unwrap();
                    }
                }
                Event::MenuEvent { .. } | Event::TrayEvent { .. } => {
                    self.handle_tray_event(event, event_loop, control_flow)
                }
                _ => (),
            }

            if self.window.is_none() && self.tray.is_none() && background_window_id.is_none() {
                *control_flow = ControlFlow::Exit;
            }
        });
    }
}
