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
    io::{
        scenes::{load_scenes, SceneEntry},
        Args, Config, StartupWith, TrayConfig,
    },
};
use shaderbg_render::{gfx::buffer::Time, scene::Scene};

pub struct State {
    pub config: Config,
    pub window_open: bool,
    pub tray_open: bool,
    pub background_open: bool,
    pub scenes: Box<[SceneEntry]>,
    current_scene: Option<usize>,
}

impl State {
    pub fn new(args: Args, mut config: Config) -> State {
        let scenes = load_scenes(config.scene_dir.clone(), config.settings_dir.clone());

        // find current scene
        let mut current_scene = None;

        // 1. check args
        if let Some(scene_name) = args.scene {
            current_scene = scenes
                .iter()
                .position(|entry| entry.name == scene_name.clone().into());
        }

        // 2. check config
        if current_scene.is_none() {
            if let Some(scene) = config.scene.clone() {
                current_scene = scenes
                    .iter()
                    .position(|entry| entry.name == scene.clone().into());
            }
        }

        // 3. check config recent
        if current_scene.is_none() {
            let most_recent_scene = config
                .recent_scenes
                .iter()
                .max_by_key(|search| search.last_used);

            if let Some(most_recent_scene) = most_recent_scene {
                current_scene = scenes
                    .iter()
                    .position(|entry| entry.name == most_recent_scene.scene.clone().into());
            }
        }

        if let Some(current_scene) = current_scene {
            let name = scenes[current_scene].name.clone().into_string();
            config.scene = Some(name.clone());
            config.push_recent_scene(name);
        }

        // other state
        let window_open =
            !args.no_window && (!args.system_startup || config.startup_with == StartupWith::Window);

        let tray_open = args.tray.unwrap_or(
            config.tray_config == TrayConfig::Enabled
                || args.system_startup && config.startup_with == StartupWith::Tray,
        );

        let background_open = args
            .background
            .unwrap_or(args.system_startup && config.startup_background);

        State {
            window_open,
            tray_open,
            background_open,
            config: config,
            scenes,
            current_scene,
        }
    }

    pub fn scene(&self) -> Option<&Scene> {
        match self.current_scene {
            Some(index) => {
                if index < self.scenes.len() {
                    Some(&self.scenes[index].scene)
                } else {
                    None
                }
            }
            None => None,
        }
    }

    pub fn scene_name(&self) -> Option<&str> {
        match self.current_scene {
            Some(index) => {
                if index < self.scenes.len() {
                    Some(&self.scenes[index].name)
                } else {
                    None
                }
            }
            None => None,
        }
    }

    pub fn scene_mut(&mut self) -> Option<&mut Scene> {
        match self.current_scene {
            Some(index) => {
                if index < self.scenes.len() {
                    Some(&mut self.scenes[index].scene)
                } else {
                    None
                }
            }
            None => None,
        }
    }

    pub fn get_scene(&self, scene: String) -> Option<&Scene> {
        self.scenes
            .iter()
            .find(|entry| entry.name == scene.clone().into())
            .map(|entry| &entry.scene)
    }

    pub fn current_scene(&self) -> Option<usize> {
        self.current_scene
    }

    pub fn set_scene(&mut self, scene: String) -> bool {
        let index = self
            .scenes
            .iter()
            .position(|entry| entry.name == scene.clone().into());

        if let Some(index) = index {
            self.current_scene = Some(index);
            true
        } else {
            false
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
    time: Arc<RwLock<Time>>,
    owner: AppEventSender,
    pub app_tx: Sender<AppMessage>,
}

impl AppState {
    pub fn build(
        state: Arc<RwLock<State>>,
        time: Arc<RwLock<Time>>,
        app_tx: Sender<AppMessage>,
        owner: AppEventSender,
    ) -> AppState {
        AppState {
            state,
            time,
            app_tx,
            owner,
        }
    }

    pub fn get(&self) -> RwLockReadGuard<'_, State> {
        self.state.read().unwrap()
    }

    pub fn get_time(&self) -> RwLockReadGuard<'_, Time> {
        self.time.read().unwrap()
    }

    pub fn send(&self, event: AppEvent) -> Result<(), SendError<AppMessage>> {
        self.app_tx.send((event, self.owner.clone()))
    }

    pub fn clone_for(&self, owner: AppEventSender) -> AppState {
        AppState {
            state: self.state.clone(),
            time: self.time.clone(),
            owner,
            app_tx: self.app_tx.clone(),
        }
    }
}
