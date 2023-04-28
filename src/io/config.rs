/*
 * config, as well as file read / save
 */

use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "config.toml";

#[derive(Deserialize, Serialize)]
pub struct RecentScene {
    scene: String,
    last_used: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
    pub background: bool,
    pub window: bool,
    pub tray: bool,
    pub scene: Option<String>,
    pub scene_dir: std::path::PathBuf,
    pub launch_on_startup: bool,
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
}
