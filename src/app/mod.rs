/*
 * app module
 *
 * the app is the 'main' thread which handles message passing between the
 * window / tray thread, background thread and IO thread.
 *
 * the real main thread is WindowThread, which processes the Tao EventLoop
 */
use std::sync::{mpsc, Arc, RwLock};
use tao::{event::Event, event_loop::EventLoopProxy};

use crate::{
    gfx::buffer::Time,
    io::{Args, Config},
    scene::Scene,
};

mod background;
mod state;
mod thread;
mod tray;
mod window;
pub use background::*;
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
    scene: Scene,
    proxy: EventLoopProxy<WindowEvent>,
) -> (AppState, std::thread::JoinHandle<()>) {
    let (tx, rx) = mpsc::channel::<AppMessage>();

    let app_tx = tx.clone();
    let state = Arc::new(RwLock::new(State {
        config: config.clone(),
        window_open: args.window.unwrap_or(config.window),
        tray_open: args.tray.unwrap_or(config.tray),
        background_open: true,
        scene,
        time: Time::new(),
    }));
    let app_state = AppState::build(state.clone(), app_tx, AppEventSender::Window);
    let return_state = app_state.clone();

    let mut background_handle: Option<std::thread::JoinHandle<()>> = None;
    let mut background_channel: Option<mpsc::Sender<BackgroundEvent>> = None;

    let handle = std::thread::spawn(move || {
        /*
        if proxy.send_event(WindowEvent::).is_err() {
            println!("failed to send windowevent");
        } */

        loop {
            match rx.recv() {
                Ok((event, _sender)) => {
                    //println!("{:?} from {:?}", event, sender);
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
                            proxy
                                .send_event(WindowEvent::CloseBackgroundWindow)
                                .unwrap();
                            background_channel.take();
                            if let Some(handle) = background_handle.take() {
                                handle.join().unwrap();
                            }
                        }
                        AppEvent::Update(dt) => {
                            if let Ok(mut state) = state.write() {
                                state.time.update_time(dt);
                            }
                        }
                        AppEvent::EventLoopReady => {
                            if state.read().unwrap().background_open {
                                proxy
                                    .send_event(WindowEvent::CreateBackgroundWindow)
                                    .unwrap();
                            }
                        }
                        AppEvent::EventLoopQuit => {
                            break;
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
