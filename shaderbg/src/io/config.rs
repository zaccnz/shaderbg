/*
 * config, as well as file read / save
 */

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const CONFIG_FILE: &str = "config.toml";
const MAX_RECENT_SCENES: usize = 10;

#[derive(Debug, Deserialize, Serialize)]
pub struct RecentScene {
    pub scene: String,
    pub last_used: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub background: bool,
    pub window: bool,
    pub tray: bool,
    pub scene: Option<String>,
    pub scene_dir: std::path::PathBuf,
    pub launch_on_startup: bool,
    pub recent_scenes: VecDeque<RecentScene>,
}

impl Config {
    pub fn default() -> Config {
        Config {
            background: false,
            window: true,
            tray: false,
            scene: None,
            scene_dir: std::path::PathBuf::from("./scene/"),
            launch_on_startup: false,
            recent_scenes: VecDeque::new(),
        }
    }

    pub fn load() -> Result<Config, String> {
        let config_string = match std::fs::read_to_string(CONFIG_FILE) {
            Ok(str) => str,
            Err(e) => return Err(e.to_string()),
        };

        let config: Config = match toml::from_str(&config_string) {
            Ok(cfg) => cfg,
            Err(e) => return Err(e.to_string()),
        };

        Ok(config)
    }

    pub fn save(&self) -> Result<(), String> {
        let config_string = match toml::to_string(&self) {
            Ok(str) => str,
            Err(e) => return Err(e.to_string()),
        };

        match std::fs::write(CONFIG_FILE, config_string) {
            Ok(()) => Ok(()),
            Err(e) => return Err(e.to_string()),
        }
    }

    pub fn push_recent_scene(&mut self, scene: String) {
        let existing = self
            .recent_scenes
            .iter()
            .position(|entry| entry.scene == scene.clone());

        if let Some(existing) = existing {
            self.recent_scenes.remove(existing);
        }

        self.recent_scenes.push_front(RecentScene {
            scene,
            last_used: chrono::Utc::now(),
        });

        if self.recent_scenes.len() > MAX_RECENT_SCENES {
            self.recent_scenes.pop_back();
        }
    }
}
