/*
 * Window thread.  Runs the Event Loop and handles all WindowEvent messages
 */
use tao::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
};

use crate::{
    app::{AppEvent, AppEventSender, AppState, Background, MenuBuilder, Tray, Window, Windows},
    io::UiTheme,
};
use shaderbg_render::scene::io::setting::SettingValue;

#[derive(Debug)]
pub enum WindowEvent {
    StartWindow,
    StartTray,
    StopTray,
    StartBackground,
    StopBackground,
    SettingUpdated(String, SettingValue),
    OpenUiWindow(Windows),
    UpdateTheme(UiTheme),
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
        app_state: &AppState,
    ) {
        let open = if let Some(window) = self.window.as_mut() {
            window.handle(event)
        } else {
            true
        };

        if !open {
            app_state.send(AppEvent::WindowStateChange(false)).unwrap();
            let window = self.window.take();
            window.unwrap().will_close(event_loop);
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
        let mut menu_builder = MenuBuilder::new(app_state.clone());

        event_loop.run(move |event, event_loop, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::NewEvents(StartCause::Init) => {
                    // THREAD START
                    #[cfg(target_os = "macos")]
                    {
                        use tao::platform::macos::{
                            ActivationPolicy, EventLoopWindowTargetExtMacOS,
                        };
                        event_loop.set_activation_policy_at_runtime(ActivationPolicy::Accessory);
                    }
                    app_state.send(AppEvent::EventLoopReady).unwrap();
                }
                Event::LoopDestroyed => {
                    // THREAD KILLED
                    if let Some(window) = self.window.take() {
                        window.will_close(event_loop);
                    }
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

                        app_state.send(AppEvent::WindowStateChange(true)).unwrap();
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
                    WindowEvent::Quit => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::SettingUpdated(key, value) => {
                        if let Some(mut window) = self.window.take() {
                            window.update_setting(key, value);
                            self.window = Some(window);
                        }
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
                    WindowEvent::UpdateTheme(theme) => {
                        if let Some(window) = self.window.as_mut() {
                            window.update_theme(theme);
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
                        if window_id == id {
                            target = WindowEventTarget::Background;
                        }
                    }

                    match target {
                        WindowEventTarget::Window => {
                            self.handle_window_event(event, event_loop, &app_state);
                        }
                        WindowEventTarget::Background => {
                            if let Some(event) = event.to_static() {
                                app_state.send(AppEvent::BackgroundEvent(event)).unwrap();
                            }
                        }
                        _ => {
                            eprintln!(
                                "Window event has no target (window_id={:?}), {:?}",
                                window_id, &event
                            );
                        }
                    }
                }
                Event::RedrawEventsCleared => {
                    self.handle_window_event(event, event_loop, &app_state);
                    if background_window_id.is_some() {
                        app_state
                            .send(AppEvent::BackgroundEvent(Event::RedrawEventsCleared))
                            .unwrap();
                    }
                }
                Event::MainEventsCleared => {
                    self.handle_window_event(event, event_loop, &app_state);
                    if background_window_id.is_some() {
                        app_state
                            .send(AppEvent::BackgroundEvent(Event::MainEventsCleared))
                            .unwrap();
                    }
                }
                Event::MenuEvent { .. } => {
                    menu_builder.handle_event(event);
                }
                Event::TrayEvent { .. } => {}
                _ => (),
            }
        });
    }
}
