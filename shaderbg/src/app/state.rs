/*
 * State shared between all threads
 * this is read only for all threads except the App thread, which will modify the state
 * based on events it receives.  Other threads have to send events to cause a state
 * change.
 */
use std::sync::{
    mpsc::{SendError, Sender},
    Arc, RwLock, RwLockReadGuard,
};

use crate::{
    app::{AppEvent, AppEventSender, AppMessage},
    io::{Args, Config},
};
use shaderbg_render::{
    gfx::buffer::{ShaderToy, Time},
    scene::Scene,
};

pub struct State {
    pub config: Config,
    pub window_open: bool,
    pub tray_open: bool,
    pub background_open: bool,
    pub scene: Scene,
    pub time: Time,
}

impl State {
    // todo: try and find scene in some list of loaded scenes
    // return error if scene not found
    pub fn new(args: Args, config: Config, scene: Scene) -> State {
        State {
            config: config.clone(),
            window_open: args.window.unwrap_or(config.window),
            tray_open: args.tray.unwrap_or(config.tray),
            background_open: args.background.unwrap_or(config.background),
            scene,
            time: Time::new(),
        }
    }
}

/*
 * AppState is the state of the app wrapped in Arc<RwLock> so it can be passed
 * around different threads, an owner (for event sending) and a Sender to send
 * messages back to 'main' app thread
 */
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

    pub fn get(&self) -> RwLockReadGuard<'_, State> {
        self.state.read().unwrap()
    }

    pub fn send(&self, event: AppEvent) -> Result<(), SendError<AppMessage>> {
        self.app_tx.send((event, self.owner.clone()))
    }

    pub fn clone_for(&self, owner: AppEventSender) -> AppState {
        AppState {
            state: self.state.clone(),
            owner,
            app_tx: self.app_tx.clone(),
        }
    }
}
