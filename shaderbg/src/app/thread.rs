/*
 * Window thread.  Runs the Event Loop and handles all WindowEvent messages
 */
use std::time::Instant;
use tao::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
};

use crate::app::{
    AppEvent, AppEventSender, AppState, Background, MenuBuilder, Tray, Window, Windows,
};
use shaderbg_render::scene::Setting;

#[derive(Debug)]
pub enum WindowEvent {
    StartWindow,
    StopWindow,
    StartTray,
    StopTray,
    StartBackground,
    StopBackground,
    SettingUpdated(String, Setting),
    OpenUiWindow(Windows),
    RebuildMenus,
    SceneChanged,
    Quit,
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
        let event_loop = EventLoop::<WindowEvent>::with_user_event();

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

    pub fn run(
        mut self,
        event_loop: EventLoop<WindowEvent>,
        app_state: AppState,
        handle: std::thread::JoinHandle<()>,
    ) {
        let mut handle = Some(handle);
        let mut background_window_id = None;
        let mut last_frame = Instant::now();
        let mut menu_builder = MenuBuilder::new(app_state.clone());

        event_loop.run(move |event, event_loop, control_flow| {
            *control_flow = ControlFlow::Wait;
            let mut started = false;
            match event {
                Event::NewEvents(StartCause::Init) => {
                    #[cfg(target_os = "macos")]
                    {
                        use tao::platform::macos::{
                            ActivationPolicy, EventLoopWindowTargetExtMacOS,
                        };
                        event_loop.set_activation_policy_at_runtime(ActivationPolicy::Accessory);
                    }
                    app_state.send(AppEvent::EventLoopReady).unwrap();
                    started = true;
                }
                Event::LoopDestroyed => {
                    self.tray.take();
                    app_state.send(AppEvent::EventLoopQuit).unwrap();
                    handle.take().unwrap().join().unwrap();
                }
                Event::UserEvent(window_event) => match window_event {
                    WindowEvent::StartWindow => {
                        if self.window.is_some() {
                            eprintln!("Cannot open window - already open");
                        } else {
                            self.window = Some(Window::build(
                                event_loop,
                                app_state.clone(),
                                &mut menu_builder,
                            ));
                        }
                    }
                    WindowEvent::StopWindow => {
                        self.window.take();
                    }
                    WindowEvent::StartTray => {
                        if self.tray.is_some() {
                            eprintln!("Cannot start tray - already started");
                        } else {
                            self.tray = Some(Tray::build(event_loop, &mut menu_builder));
                            app_state.send(AppEvent::TrayStateChange(true)).unwrap();
                        }
                    }
                    WindowEvent::StopTray => {
                        self.tray.take();
                        app_state.send(AppEvent::TrayStateChange(false)).unwrap();
                    }
                    WindowEvent::StartBackground => {
                        let background = Background::new(
                            event_loop,
                            app_state.clone_for(AppEventSender::Background),
                        );

                        background_window_id = Some(background.window.id());

                        app_state
                            .send(AppEvent::BackgroundCreated(background))
                            .unwrap();
                    }
                    WindowEvent::StopBackground => {
                        background_window_id.take();
                    }
                    WindowEvent::SettingUpdated(key, value) => {
                        if let Some(mut window) = self.window.take() {
                            window.update_setting(key, value);
                            self.window = Some(window);
                        }
                    }
                    WindowEvent::Quit => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::OpenUiWindow(ui_window) => {
                        if self.window.is_none() {
                            self.window = Some(Window::build(
                                event_loop,
                                app_state.clone(),
                                &mut menu_builder,
                            ));
                        }

                        if let Some(window) = self.window.as_mut() {
                            window.open_ui_window(ui_window);
                        }
                    }
                    WindowEvent::RebuildMenus => {
                        if let Some(window) = self.window.as_mut() {
                            window.rebuild_menus(&mut menu_builder);
                        }
                        if let Some(tray) = self.tray.as_mut() {
                            tray.rebuild_menus(&mut menu_builder);
                        }
                    }
                    WindowEvent::SceneChanged => {
                        if let Some(window) = self.window.as_mut() {
                            window.scene_changed();
                        }
                    }
                },
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
                    self.handle_window_event(event, event_loop, control_flow);
                    if background_window_id.is_some() {
                        app_state
                            .send(AppEvent::BackgroundEvent(Event::MainEventsCleared))
                            .unwrap();
                    }

                    let now = Instant::now();
                    app_state
                        .send(AppEvent::Update((now - last_frame).as_secs_f64()))
                        .unwrap();
                    last_frame = now;
                }
                Event::MenuEvent { .. } => {
                    menu_builder.handle_event(event);
                }
                Event::TrayEvent { .. } => {}
                _ => (),
            }

            if !started
                && self.window.is_none()
                && self.tray.is_none()
                && background_window_id.is_none()
            {
                *control_flow = ControlFlow::Exit;
            }
        });
    }
}
