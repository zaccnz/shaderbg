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

use crate::{
    app::timer::TimerMessage,
    io::{Args, Config, ConfigUpdate, TrayConfig},
};
use shaderbg_render::{gfx::buffer::Time, scene::io::setting::SettingValue};

mod background;
mod menu;
mod state;
mod thread;
pub mod timer;
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
    ShouldClose,
    Update(f64),
    Window(ThreadEvent),
    WindowStateChange(bool),
    TrayStateChange(bool),
    BackgroundCreated(Background),
    BackgroundEvent(Event<'static, ThreadEvent>),
    BackgroundClosed(bool),
    SceneSettingsSaved,
    SettingUpdated(String, SettingValue),
    ConfigUpdated(Box<[ConfigUpdate]>),
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
    proxy: EventLoopProxy<ThreadEvent>,
) -> (AppState, std::thread::JoinHandle<()>) {
    let (tx, rx) = mpsc::channel::<AppMessage>();

    let app_tx = tx.clone();
    let state = Arc::new(RwLock::new(State::new(args, config)));
    let time = Arc::new(RwLock::new(Time::new()));
    let app_state = AppState::build(state.clone(), time.clone(), app_tx, AppEventSender::Window);
    let return_state = app_state.clone();

    let mut background_handle: Option<std::thread::JoinHandle<()>> = None;
    let mut background_channel: Option<mpsc::Sender<BackgroundEvent>> = None;

    let started = SystemTime::now();

    let (timer_tx, timer_rx) = mpsc::channel();
    let mut timer_handle = Some(timer::run(app_state.clone(), timer_rx));

    let handle = std::thread::spawn(move || {
        // let mut was_empty = None;
        loop {
            let event = match rx.recv() {
                Ok((event, _sender)) => event,
                Err(e) => {
                    eprintln!("AppEvent RecvError {:?}", e);
                    continue;
                }
            };

            match event {
                AppEvent::Window(event) => {
                    let close_tray = match &event {
                        ThreadEvent::StartWindow | ThreadEvent::OpenUiWindow(_) => {
                            let state = state.read().unwrap();
                            state.config.tray_config != TrayConfig::Enabled && state.tray_open
                        }
                        _ => false,
                    };
                    proxy.send_event(event).unwrap();

                    if close_tray {
                        proxy.send_event(ThreadEvent::StopTray).unwrap();
                    }
                }
                AppEvent::ShouldClose => {
                    proxy.send_event(ThreadEvent::Quit).unwrap();
                    timer_tx.send(TimerMessage::Quit).ok();
                    if let Some(handle) = timer_handle.take() {
                        handle.join().unwrap();
                    }
                }
                AppEvent::WindowStateChange(value) => {
                    state.write().unwrap().window_open = value;
                    timer_tx.send(TimerMessage::WindowChange(value)).unwrap();
                }
                AppEvent::TrayStateChange(value) => {
                    state.write().unwrap().tray_open = value;
                    timer_tx.send(TimerMessage::TrayChange(value)).unwrap();
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

                    {
                        let mut state = state.write().unwrap();
                        state.background_open = true;
                        state.config.background_enabled = true;
                        if let Err(error) = state.config.save() {
                            eprintln!("Error saving config {:?}", error);
                        }
                    }
                    proxy.send_event(ThreadEvent::RebuildMenus).unwrap();
                    timer_tx.send(TimerMessage::BackgroundChange(true)).unwrap();
                }
                AppEvent::BackgroundEvent(event) => {
                    if let Some(channel) = background_channel.take() {
                        channel.send(BackgroundEvent::TaoEvent(event)).unwrap();
                        background_channel = Some(channel);
                    }
                }
                AppEvent::BackgroundClosed(manual) => {
                    proxy.send_event(ThreadEvent::StopBackground).unwrap();
                    let channel = background_channel.take();
                    if let Some(handle) = background_handle.take() {
                        if !handle.is_finished() {
                            if let Some(channel) = channel {
                                channel.send(BackgroundEvent::Stop).unwrap();
                            }
                        }
                        handle.join().unwrap();
                    }

                    {
                        let mut state = state.write().unwrap();
                        state.background_open = false;
                        if manual {
                            state.config.background_enabled = false;
                            if let Err(error) = state.config.save() {
                                eprintln!("Error saving config {:?}", error);
                            }
                        }
                    }
                    proxy.send_event(ThreadEvent::RebuildMenus).unwrap();
                    timer_tx
                        .send(TimerMessage::BackgroundChange(false))
                        .unwrap();
                }
                AppEvent::Update(dt) => {
                    let now = SystemTime::now()
                        .duration_since(started)
                        .unwrap()
                        .as_millis() as u32;
                    if let Ok(mut time) = time.write() {
                        time.update_time(now, dt);
                    }
                }
                AppEvent::EventLoopReady => {
                    let (window_open, tray_open, background_open) = {
                        let state = app_state.get();
                        (state.window_open, state.tray_open, state.background_open)
                    };
                    if window_open {
                        proxy.send_event(ThreadEvent::StartWindow).unwrap();
                    }
                    if tray_open {
                        proxy.send_event(ThreadEvent::StartTray).unwrap();
                    }
                    if background_open {
                        proxy.send_event(ThreadEvent::StartBackground).unwrap();
                    }
                }
                AppEvent::EventLoopQuit => {
                    break;
                }
                AppEvent::SceneSettingsSaved => {
                    if let Ok(state) = state.read() {
                        if let Some(scene) = state.scene() {
                            scene
                                .settings
                                .save(
                                    state
                                        .config
                                        .settings_dir
                                        .join(format!("{}.toml", state.scene_name().unwrap())),
                                )
                                .unwrap();
                        }
                    }
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
                        .send_event(ThreadEvent::SettingUpdated(key.clone(), setting.clone()))
                        .unwrap();

                    if let Some(background) = background_channel.as_ref() {
                        background
                            .send(BackgroundEvent::SettingUpdated(key, setting))
                            .unwrap();
                    }
                }
                AppEvent::SetScene(scene) => {
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
                        proxy.send_event(ThreadEvent::RebuildMenus).unwrap();
                        proxy.send_event(ThreadEvent::SceneChanged).unwrap();

                        if let Some(background) = background_channel.as_ref() {
                            background.send(BackgroundEvent::SceneChanged).unwrap();
                        }
                    }
                }
                AppEvent::ConfigUpdated(updates) => {
                    if let Ok(mut state) = state.write() {
                        for update in updates.into_vec() {
                            match &update {
                                ConfigUpdate::Theme(theme) => proxy
                                    .send_event(ThreadEvent::UpdateTheme(theme.clone()))
                                    .unwrap(),
                                _ => {}
                            }
                            state.config.update(update);
                        }

                        if let Err(e) = state.config.save() {
                            eprintln!("Error saving config: {}", e);
                        }
                    }
                }
            };
        }

        if let Ok(state) = state.read() {
            if let Err(e) = state.config.save() {
                eprintln!("Error saving config: {}", e);
            }
        }
    });

    (return_state, handle)
}
