/*
 * app module
 *
 * the app is the 'main' thread which handles message passing between the
 * window / tray thread, background thread and IO thread.
 *
 * the real main thread is WindowThread, which processes the Tao EventLoop
 */
use std::{
    sync::{mpsc, Arc, RwLock},
    time::SystemTime,
};
use tao::{event::Event, event_loop::EventLoopProxy};

use crate::io::{Args, Config};
use shaderbg_render::scene::Setting;

mod background;
mod menu;
mod state;
mod thread;
mod tray;
mod window;
pub use background::*;
pub use menu::*;
pub use state::*;
pub use thread::*;
pub use tray::*;
pub use window::*;

#[derive(Debug)]
pub enum AppEvent {
    EventLoopReady,
    EventLoopQuit,
    Update(f64),
    Window(WindowEvent),
    TrayStateChange(bool),
    BackgroundCreated(Background),
    BackgroundEvent(Event<'static, WindowEvent>),
    BackgroundClosed,
    SettingUpdated(String, Setting),
    SetScene(String),
}

#[derive(Clone, Debug)]
pub enum AppEventSender {
    Window,
    Background,
}

pub type AppMessage = (AppEvent, AppEventSender);

// we want to return the pipe for receiving events on our window
pub fn start_main(
    args: Args,
    config: Config,
    proxy: EventLoopProxy<WindowEvent>,
) -> (AppState, std::thread::JoinHandle<()>) {
    let (tx, rx) = mpsc::channel::<AppMessage>();

    let app_tx = tx.clone();
    let state = Arc::new(RwLock::new(State::new(args, config)));
    let app_state = AppState::build(state.clone(), app_tx, AppEventSender::Window);
    let return_state = app_state.clone();

    let mut background_handle: Option<std::thread::JoinHandle<()>> = None;
    let mut background_channel: Option<mpsc::Sender<BackgroundEvent>> = None;

    let started = SystemTime::now();

    let handle = std::thread::spawn(move || {
        loop {
            match rx.recv() {
                Ok((event, _sender)) => {
                    match event {
                        AppEvent::Window(event) => {
                            proxy.send_event(event).unwrap();
                        }
                        AppEvent::TrayStateChange(value) => {
                            state.write().unwrap().tray_open = value;
                        }
                        AppEvent::BackgroundCreated(background) => {
                            if background_handle.is_some() {
                                eprintln!("Cannot create new background, already running");
                                drop(background);
                                return;
                            }

                            let (tx, rx) = mpsc::channel();
                            background_channel = Some(tx);

                            background_handle = Some(std::thread::spawn(move || {
                                background.run(rx);
                            }));
                        }
                        AppEvent::BackgroundEvent(event) => {
                            if let Some(channel) = background_channel.take() {
                                channel.send(BackgroundEvent::TaoEvent(event)).unwrap();
                                background_channel = Some(channel);
                            }
                        }
                        AppEvent::BackgroundClosed => {
                            proxy.send_event(WindowEvent::StopBackground).unwrap();
                            background_channel.take();
                            if let Some(handle) = background_handle.take() {
                                handle.join().unwrap();
                            }
                        }
                        AppEvent::Update(dt) => {
                            if let Ok(mut state) = state.write() {
                                let now = SystemTime::now()
                                    .duration_since(started)
                                    .unwrap()
                                    .as_millis() as u32;
                                state.time.update_time(now, dt);
                            }
                        }
                        AppEvent::EventLoopReady => {
                            let (window_open, tray_open, background_open) = {
                                let state = app_state.get();
                                (state.window_open, state.tray_open, state.background_open)
                            };
                            if window_open {
                                proxy.send_event(WindowEvent::StartWindow).unwrap();
                            }
                            if tray_open {
                                proxy.send_event(WindowEvent::StartTray).unwrap();
                            }
                            if background_open {
                                proxy.send_event(WindowEvent::StartBackground).unwrap();
                            }
                        }
                        AppEvent::EventLoopQuit => {
                            break;
                        }
                        AppEvent::SettingUpdated(key, setting) => {
                            if let Ok(mut state) = state.write() {
                                let key = key.clone();
                                let setting = setting.clone();
                                if let Some(scene) = state.scene_mut() {
                                    scene.settings.update(&key, setting);
                                }
                            }
                            proxy
                                .send_event(WindowEvent::SettingUpdated(
                                    key.clone(),
                                    setting.clone(),
                                ))
                                .unwrap();

                            if let Some(background) = background_channel.as_ref() {
                                background
                                    .send(BackgroundEvent::SettingUpdated(key, setting))
                                    .unwrap();
                            }
                        }
                        AppEvent::SetScene(scene) => {
                            println!("setting scene {}", scene);
                            let changed = if let Ok(mut state) = state.write() {
                                if state.set_scene(scene.clone()) {
                                    state.config.scene = Some(scene.clone());
                                    state.config.push_recent_scene(scene);
                                    true
                                } else {
                                    false
                                }
                            } else {
                                false
                            };

                            if changed {
                                proxy.send_event(WindowEvent::RebuildMenus).unwrap();
                                proxy.send_event(WindowEvent::SceneChanged).unwrap();

                                if let Some(background) = background_channel.as_ref() {
                                    background.send(BackgroundEvent::SceneChanged).unwrap();
                                }
                            }
                        }
                    };
                }
                _ => {}
            };
        }

        if let Err(e) = app_state.get().config.save() {
            eprintln!("error saving state: {}", e);
        }
    });

    (return_state, handle)
}
