use egui::{Ui, WidgetText};

use crate::app::AppState;

#[derive(Clone, Debug, PartialEq)]
enum UiTheme {
    Light,
    Dark,
    System,
}

#[derive(Clone, PartialEq)]
enum StartupWith {
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

#[derive(Clone, PartialEq)]
enum TrayState {
    Enabled,
    MinimizeTo,
    CloseTo,
    Disabled,
}

impl std::fmt::Debug for TrayState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Enabled => write!(f, "Enabled"),
            Self::MinimizeTo => write!(f, "Minimize to tray"),
            Self::CloseTo => write!(f, "Close to tray"),
            Self::Disabled => write!(f, "Disabled"),
        }
    }
}

pub struct Settings {
    #[allow(dead_code)]
    app_state: AppState,
    scene_dir: String,
    preference_dir: String,
    launch_on_startup: bool,
    background: bool,
    ui_theme: UiTheme,
    startup_with: StartupWith,
    tray_state: TrayState,
}

impl Settings {
    pub fn new(app_state: AppState) -> Settings {
        let (scene_dir, background) = {
            let config = &app_state.get().config;

            (
                config.scene_dir.to_str().unwrap().to_string(),
                config.background,
            )
        };

        Settings {
            app_state,
            scene_dir: scene_dir.clone(),
            preference_dir: ".".to_string(),
            launch_on_startup: false,
            background,
            ui_theme: UiTheme::Dark,
            startup_with: StartupWith::Tray,
            tray_state: TrayState::CloseTo,
        }
    }

    fn save(&self) {}

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
                ui.checkbox(&mut self.background, "Start background");
                if !self.background && self.startup_with == StartupWith::Neither {
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
            &mut self.tray_state,
            &[
                TrayState::Enabled,
                TrayState::MinimizeTo,
                TrayState::CloseTo,
                TrayState::Disabled,
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
        ui.label("Preference Directory");
        ui.text_edit_singleline(&mut self.preference_dir);

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button("Cancel").clicked() {
                open = false
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Save").clicked() {
                    self.save();
                }
            });
        });

        open
    }
}
