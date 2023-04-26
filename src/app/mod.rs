/*
 * app module
 *
 * the app is the 'main' thread which handles message passing between the
 * window / tray thread, background thread and IO thread.
 *
 * the real main thread is WindowThread, which processes the Tao EventLoop
 */
use std::sync::{mpsc, Arc, RwLock};
use tao::event_loop::EventLoopProxy;

mod state;
mod thread;
mod tray;
mod window;
pub use state::*;
pub use thread::*;
pub use tray::*;
pub use window::*;

#[derive(Debug)]
pub enum AppEvent {
    EventLoopReady,
    Window(WindowEvent),
    TrayStateChange(bool),
}

#[derive(Clone, Debug)]
pub enum AppEventSender {
    Window,
}

pub type AppMessage = (AppEvent, AppEventSender);

// we want to return the pipe for receiving events on our window
pub fn start_main(proxy: EventLoopProxy<WindowEvent>) -> AppState {
    let (tx, rx) = mpsc::channel::<AppMessage>();

    let app_tx = tx.clone();
    let state = Arc::new(RwLock::new(State {
        window_open: true,
        tray_open: false,
    }));
    let app_state = AppState::build(state.clone(), app_tx, AppEventSender::Window);

    std::thread::spawn(move || {
        println!("subthread spawned");
        /*
        if proxy.send_event(WindowEvent::).is_err() {
            println!("failed to send windowevent");
        } */

        loop {
            match rx.recv() {
                Ok((event, sender)) => {
                    println!("{:?} from {:?}", event, sender);
                    match event {
                        AppEvent::Window(event) => {
                            proxy.send_event(event).unwrap();
                        }
                        AppEvent::TrayStateChange(value) => {
                            state.write().unwrap().tray_open = value;
                        }
                        _ => {}
                    };
                }
                _ => {}
            };
        }
    });

    app_state
}
