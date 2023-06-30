use egui::{Ui, WidgetText};

use crate::{
    app::{AppEvent, AppState, ThreadEvent},
    io::{ConfigUpdate, StartupWith, TrayConfig, UiTheme},
};

enum SettingsError {
    SceneDir(Option<String>),
    SettingsDir(Option<String>),
}

pub struct Settings {
    app_state: AppState,
    scene_dir: String,
    settings_dir: String,
    launch_on_startup: bool,
    startup_with: StartupWith,
    startup_background: bool,
    ui_theme: UiTheme,
    tray_config: TrayConfig,
    error: Option<SettingsError>,
}

impl Settings {
    pub fn new(app_state: AppState) -> Settings {
        let (
            scene_dir,
            settings_dir,
            launch_on_startup,
            ui_theme,
            startup_with,
            startup_background,
            tray_config,
        ) = {
            let config = &app_state.get().config;

            (
                config.scene_dir.to_str().unwrap().to_string(),
                config.settings_dir.to_str().unwrap().to_string(),
                config.launch_on_startup,
                config.theme.clone(),
                config.startup_with.clone(),
                config.startup_background,
                config.tray_config.clone(),
            )
        };

        Settings {
            app_state,
            scene_dir,
            settings_dir,
            launch_on_startup,
            startup_background,
            startup_with,
            ui_theme,
            tray_config,
            error: None,
        }
    }

    fn save(&mut self) -> bool {
        let config = &self.app_state.get().config;
        let mut changes = Vec::new();

        if self.scene_dir != config.scene_dir.to_str().unwrap() {
            let path_buf = std::path::PathBuf::from(self.scene_dir.clone());
            if !path_buf.exists() {
                self.error = Some(SettingsError::SceneDir(None));
                return false;
            }
            if path_buf.is_file() {
                self.error = Some(SettingsError::SceneDir(Some(
                    "is already a file".to_string(),
                )));
                return false;
            }
            changes.push(ConfigUpdate::SceneDir(path_buf));
        }

        if self.settings_dir != config.settings_dir.to_str().unwrap() {
            let path_buf = std::path::PathBuf::from(self.settings_dir.clone());
            if !path_buf.exists() {
                self.error = Some(SettingsError::SettingsDir(None));
                return false;
            }
            if path_buf.is_file() {
                self.error = Some(SettingsError::SettingsDir(Some(
                    "is already a file".to_string(),
                )));
                return false;
            }
            changes.push(ConfigUpdate::SettingsDir(path_buf));
        }

        if self.launch_on_startup != config.launch_on_startup {
            // try install
            changes.push(ConfigUpdate::LaunchOnStartup(self.launch_on_startup));
        }

        if self.startup_with != config.startup_with {
            changes.push(ConfigUpdate::StartupWith(self.startup_with.clone()));
        }

        if self.startup_background != config.startup_background {
            changes.push(ConfigUpdate::StartupBackground(self.startup_background));
        }

        if self.ui_theme != config.theme {
            changes.push(ConfigUpdate::Theme(self.ui_theme.clone()));
        }

        if self.tray_config != config.tray_config {
            match self.tray_config {
                TrayConfig::Enabled => {
                    self.app_state
                        .send(AppEvent::Window(ThreadEvent::StartTray))
                        .unwrap();
                }
                TrayConfig::Disabled | TrayConfig::CloseTo => {
                    self.app_state
                        .send(AppEvent::Window(ThreadEvent::StopTray))
                        .unwrap();
                }
            }
            changes.push(ConfigUpdate::TrayConfig(self.tray_config.clone()));
        }

        drop(config);

        self.app_state
            .send(AppEvent::ConfigUpdated(changes.into_boxed_slice()))
            .unwrap();

        true
    }

    fn combo_box<T>(
        ui: &mut Ui,
        id: &str,
        label: Option<impl Into<WidgetText>>,
        value: &mut T,
        options: &[T],
    ) where
        T: std::fmt::Debug + PartialEq + Clone,
    {
        ui.horizontal(|ui| {
            if let Some(label) = label {
                ui.label(label);
            }
            egui::ComboBox::from_id_source(id)
                .selected_text(format!("{:?}", value))
                .show_ui(ui, |ui| {
                    for state in options {
                        ui.selectable_value(value, state.clone(), format!("{:?}", state));
                    }
                });
        });
    }

    pub fn render(&mut self, ui: &mut Ui) -> bool {
        let mut open = true;
        ui.heading("Options");
        ui.checkbox(&mut self.launch_on_startup, "Launch on system startup");

        if self.launch_on_startup {
            ui.group(|ui| {
                ui.label("On startup...");
                Self::combo_box(
                    ui,
                    "startup_with_combo_box",
                    None::<&str>,
                    &mut self.startup_with,
                    &[StartupWith::Tray, StartupWith::Window, StartupWith::Neither],
                );
                ui.checkbox(&mut self.startup_background, "Start background");
                if !self.startup_background && self.startup_with == StartupWith::Neither {
                    ui.label("The application will not open on startup");
                }
            });
            ui.add_space(5.0);
        }

        ui.add_space(5.0);
        Self::combo_box(
            ui,
            "tray_state_combo",
            Some("System Tray"),
            &mut self.tray_config,
            &[
                TrayConfig::Enabled,
                TrayConfig::CloseTo,
                TrayConfig::Disabled,
            ],
        );

        ui.add_space(10.0);
        ui.heading("Theme");
        Self::combo_box(
            ui,
            "ui_theme_combo",
            Some("UI Theme"),
            &mut self.ui_theme,
            &[UiTheme::Light, UiTheme::Dark, UiTheme::System],
        );

        ui.add_space(10.0);
        ui.heading("Paths");
        ui.label("Scene Directory");
        ui.text_edit_singleline(&mut self.scene_dir);
        if let Some(SettingsError::SceneDir(error)) = self.error.as_ref() {
            ui.label(format!(
                "Scene directory {}",
                if let Some(error) = error.as_ref() {
                    error.as_str()
                } else {
                    "does not exist"
                }
            ));
            if error.is_none() && ui.button("Create").clicked() {
                let pathbuf = std::path::PathBuf::from(self.scene_dir.clone());
                if let Err(error) = std::fs::create_dir_all(pathbuf.as_path()) {
                    self.error = Some(SettingsError::SceneDir(Some(error.to_string())));
                } else {
                    self.error.take();
                }
            }
        };
        ui.label("Settings Directory");
        ui.text_edit_singleline(&mut self.settings_dir);
        if let Some(SettingsError::SettingsDir(error)) = self.error.as_ref() {
            ui.label(format!(
                "Settings directory {}",
                if let Some(error) = error.as_ref() {
                    error.as_str()
                } else {
                    "does not exist"
                }
            ));
            if error.is_none() && ui.button("Create").clicked() {
                let pathbuf = std::path::PathBuf::from(self.settings_dir.clone());
                if let Err(error) = std::fs::create_dir_all(pathbuf.as_path()) {
                    self.error = Some(SettingsError::SettingsDir(Some(error.to_string())));
                } else {
                    self.error.take();
                }
            }
        };

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button("Cancel").clicked() {
                open = false;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Save").clicked() {
                    if self.save() {
                        open = false;
                    }
                }
            });
        });

        open
    }
}
