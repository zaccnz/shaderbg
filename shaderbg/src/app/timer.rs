use std::{
    sync::mpsc::Receiver,
    thread::JoinHandle,
    time::{Duration, Instant},
};

use crate::app::{AppEvent, AppState};

pub enum TimerMessage {
    WindowChange(bool),
    TrayChange(bool),
    BackgroundChange(bool),
    Quit,
}

const APP_IDLE_TIMEOUT: u128 = 500; // after APP_IDLE_TIMEOUT ms of no window,tray,background open shaderbg will close

pub fn run(app_state: AppState, receiver: Receiver<TimerMessage>) -> JoinHandle<()> {
    let mut last_frame = Instant::now();

    let mut window_open = false;
    let mut tray_open = false;
    let mut background_open = false;

    let mut should_close = false;
    let mut should_close_last_changed = last_frame;

    std::thread::spawn(move || loop {
        let now = Instant::now();
        app_state
            .send(AppEvent::Update((now - last_frame).as_secs_f64()))
            .ok();
        last_frame = now;

        if let Ok(message) = receiver.recv_timeout(Duration::from_millis(10)) {
            match message {
                TimerMessage::WindowChange(open) => {
                    window_open = open;
                }
                TimerMessage::TrayChange(open) => {
                    tray_open = open;
                }
                TimerMessage::BackgroundChange(open) => {
                    background_open = open;
                }
                TimerMessage::Quit => break,
            }

            if !(window_open || tray_open || background_open) {
                should_close = true;
                should_close_last_changed = last_frame;
            } else {
                should_close = false;
            }
        }

        if should_close && (now - should_close_last_changed).as_millis() > APP_IDLE_TIMEOUT {
            app_state.send(AppEvent::ShouldClose).unwrap();
            break;
        }
    })
}
