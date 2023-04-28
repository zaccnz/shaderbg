/*
 * State shared between all threads
 * this is read only by all threads except the App thread, which will modify the state
 * based on events it receives.  Other threads have to send events to cause a state
 * change
 */
use std::sync::{
    mpsc::{SendError, Sender},
    Arc, RwLock, RwLockReadGuard,
};

use crate::{
    app::{AppEvent, AppEventSender, AppMessage},
    io::Config,
};

pub struct State {
    pub config: Config,
    pub window_open: bool,
    pub tray_open: bool,
    pub background_open: bool,
}

#[derive(Clone)]
pub struct AppState {
    state: Arc<RwLock<State>>,
    owner: AppEventSender,
    pub app_tx: Sender<AppMessage>,
}

impl AppState {
    pub fn build(
        state: Arc<RwLock<State>>,
        app_tx: Sender<AppMessage>,
        owner: AppEventSender,
    ) -> AppState {
        AppState {
            state,
            app_tx,
            owner,
        }
    }

    pub fn get_state(&self) -> RwLockReadGuard<'_, State> {
        self.state.read().unwrap()
    }

    pub fn send_event(&self, event: AppEvent) -> Result<(), SendError<AppMessage>> {
        self.app_tx.send((event, self.owner.clone()))
    }

    #[allow(dead_code)]
    pub fn clone_for(&self, owner: AppEventSender) -> AppState {
        AppState {
            state: self.state.clone(),
            owner,
            app_tx: self.app_tx.clone(),
        }
    }
}
