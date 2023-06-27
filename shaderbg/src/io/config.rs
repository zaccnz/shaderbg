/*
 * config, as well as file read / save
 */

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const CONFIG_FILE: &str = "config.toml";
const MAX_RECENT_SCENES: usize = 10;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UiTheme {
    Light,
    Dark,
    System,
}

#[derive(Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupWith {
    Tray,
    Window,
    Neither,
}

impl std::fmt::Debug for StartupWith {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tray => write!(f, "Start tray"),
            Self::Window => write!(f, "Open window"),
            Self::Neither => write!(f, "Neither"),
        }
    }
}

#[derive(Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrayConfig {
    Enabled,
    CloseTo,
    Disabled,
}

impl std::fmt::Debug for TrayConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Enabled => write!(f, "Enabled"),
            Self::CloseTo => write!(f, "Close to tray"),
            Self::Disabled => write!(f, "Disabled"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RecentScene {
    pub scene: String,
    pub last_used: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug)]
pub enum ConfigUpdate {
    SceneDir(std::path::PathBuf),
    SettingsDir(std::path::PathBuf),
    StartupWith(StartupWith),
    StartupBackground(bool),
    Theme(UiTheme),
    TrayConfig(TrayConfig),
    LaunchOnStartup(bool),
}

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub scene: Option<String>,
    pub scene_dir: std::path::PathBuf,
    pub settings_dir: std::path::PathBuf,
    pub startup_with: StartupWith,
    pub startup_background: bool,
    pub theme: UiTheme,
    pub tray_config: TrayConfig,
    pub launch_on_startup: bool,
    pub recent_scenes: VecDeque<RecentScene>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            scene: None,
            scene_dir: std::path::PathBuf::from("./scene/"),
            settings_dir: std::path::PathBuf::from("./settings/"),
            theme: UiTheme::Dark,
            startup_with: StartupWith::Tray,
            startup_background: true,
            tray_config: TrayConfig::CloseTo,
            launch_on_startup: false,
            recent_scenes: VecDeque::new(),
        }
    }
}

impl Config {
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

    pub fn update(&mut self, update: ConfigUpdate) {
        match update {
            ConfigUpdate::SceneDir(dir) => {
                self.scene_dir = dir;
            }
            ConfigUpdate::SettingsDir(dir) => {
                self.settings_dir = dir;
            }
            ConfigUpdate::StartupWith(startup_with) => {
                self.startup_with = startup_with;
            }
            ConfigUpdate::StartupBackground(startup_background) => {
                self.startup_background = startup_background;
            }
            ConfigUpdate::Theme(theme) => {
                self.theme = theme;
            }
            ConfigUpdate::TrayConfig(config) => {
                self.tray_config = config;
            }
            ConfigUpdate::LaunchOnStartup(launch_on_startup) => {
                self.launch_on_startup = launch_on_startup;
            }
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
