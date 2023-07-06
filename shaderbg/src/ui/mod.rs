mod background;
mod performance;
mod settings;
pub use background::*;
pub use performance::*;
pub use settings::*;

use tao::{
    event::{Event, WindowEvent},
    window::{Theme, Window},
};

use crate::{
    app::{AppEvent, AppState, ThreadEvent, Windows},
    egui_tao,
    io::UiTheme,
};
use shaderbg_render::{
    gfx::{self, buffer::Time, ui::SceneUiResult, Gfx},
    scene::{io::setting::SettingValue, Scene, Settings as SceneSettings},
};

pub struct AppUi {
    egui_platform: egui_tao::State,
    app_state: AppState,
    scene: Option<gfx::ui::Scene>,
    browser: Option<gfx::ui::Browser>,
    background: Option<Background>,
    settings: Option<Settings>,
    performance: Option<Performance>,
}

impl AppUi {
    pub fn new(ui: &gfx::ui::Ui, window: &Window, app_state: AppState) -> AppUi {
        let theme = { app_state.get().config.theme.clone() };

        let visuals = match theme {
            UiTheme::System => {
                if window.theme() == Theme::Dark {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                }
            }
            UiTheme::Light => egui::Visuals::light(),
            UiTheme::Dark => egui::Visuals::dark(),
        };

        ui.context().set_visuals(visuals);

        let mut egui_platform = egui_tao::State::new(&window);
        egui_platform.set_pixels_per_point(window.scale_factor() as f32);

        AppUi {
            egui_platform,
            app_state,
            scene: None,
            browser: None,
            background: None,
            settings: None,
            performance: None,
        }
    }

    pub fn handle_event(&mut self, event: &Event<ThreadEvent>, context: &egui::Context) {
        if let Event::WindowEvent { event, .. } = event {
            if let WindowEvent::ScaleFactorChanged { scale_factor, .. } = event {
                self.egui_platform
                    .set_pixels_per_point(*scale_factor as f32);
            }
            let _ = self.egui_platform.on_event(context, event);
        }
    }

    pub fn handle_full_output(
        &mut self,
        full_output: egui::PlatformOutput,
        window: &Window,
        context: &egui::Context,
    ) {
        self.egui_platform
            .handle_platform_output(window, context, full_output);
    }

    pub fn get_input(&mut self, window: &Window) -> (f32, egui::RawInput) {
        (
            self.egui_platform.pixels_per_point(),
            self.egui_platform.take_egui_input(window),
        )
    }

    pub fn update_browser(&mut self, gfx: &mut Gfx, time: Time) {
        if let Some(browser) = self.browser.as_mut() {
            browser.update_previews(
                gfx.ui.as_mut().unwrap().renderer_mut(),
                &gfx.queue,
                &mut gfx.device,
                time,
            );
        }
    }

    pub fn update_scene_ui(&mut self, scene: &Scene) {
        if self.scene.is_some() {
            self.scene = Some(gfx::ui::Scene::new(&scene.descriptor, &scene.settings));
        }
    }

    fn scene_ui(scene: &Scene) -> Option<gfx::ui::Scene> {
        Some(gfx::ui::Scene::new(&scene.descriptor, &scene.settings))
    }

    fn browser(app_state: &AppState, gfx: &Gfx) -> Option<gfx::ui::Browser> {
        Some(gfx::ui::Browser::new(
            app_state
                .get()
                .scenes
                .iter()
                .map(|entry| (entry.name.clone().to_string(), &entry.scene))
                .collect(),
            &gfx.device,
        ))
    }

    fn background(app_state: &AppState) -> Option<Background> {
        Some(Background::new(app_state.clone()))
    }

    fn settings(app_state: &AppState) -> Option<Settings> {
        Some(Settings::new(app_state.clone()))
    }

    fn performance() -> Option<Performance> {
        Some(Performance::new())
    }

    fn main_menu(&mut self, ui: &mut egui::Ui, gfx: &Gfx) {
        let mut window = None;

        if self.app_state.get().scene().is_some() {
            ui.label("Scene");
            if ui.button("Pause").clicked() {}
            if ui.button("Reload").clicked() {}
            if ui.button("Scene Settings").clicked() {
                window = Some(Windows::SceneSettings);
            }
        } else {
            ui.heading("No Scene Loaded");
        }
        ui.label("App");
        if ui.button("Scene Browser").clicked() {
            window = Some(Windows::SceneBrowser);
        }
        if ui.button("Configure Background").clicked() {
            window = Some(Windows::ConfigureBackground);
        }
        if ui.button("Settings").clicked() {
            window = Some(Windows::Settings);
        }
        if ui.button("Performance").clicked() {
            window = Some(Windows::Performance);
        }

        if let Some(window) = window {
            self.open_window(window, gfx);
        }
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        gfx: &Gfx,
        settings: Option<&SceneSettings>,
        changes: &mut Vec<(String, SettingValue)>,
        scene_ui_result: &mut SceneUiResult,
        browser_reload: &mut bool,
    ) {
        let title = if let Some(scene) = self.app_state.get().scene() {
            format!("Menu - {}", scene.descriptor.meta.name)
        } else {
            "Menu".to_string()
        };

        egui::Window::new(title)
            .movable(false)
            .resizable(false)
            .id("shaderbg".into())
            .show(ctx, |ui: &mut egui::Ui| {
                self.main_menu(ui, gfx);
            });

        if let Some(scene) = self.scene.as_mut() {
            let mut open = true;
            egui::Window::new("Scene Settings")
                .open(&mut open)
                .resizable(false)
                .show(ctx, |ui| {
                    if let Some(settings) = settings {
                        *scene_ui_result = scene.render(ui, settings, changes);
                    } else {
                        ui.heading("An error occurred");
                    }
                });
            if !open {
                *scene_ui_result = SceneUiResult::Closed;
            }
        }

        match *scene_ui_result {
            SceneUiResult::Closed | SceneUiResult::Saved => {
                self.scene.take();
            }
            _ => (),
        }

        let mut browser_open = true;
        if let Some(browser) = self.browser.as_ref() {
            egui::Window::new("Scene Browser")
                .open(&mut browser_open)
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    let scene = browser.render(
                        ui,
                        self.app_state.get().current_scene(),
                        Some(browser_reload),
                    );

                    if let Some(scene) = scene {
                        self.app_state
                            .send(AppEvent::SetScene(
                                self.app_state.get().scenes[scene].name.to_string(),
                            ))
                            .unwrap();
                    }
                });
        }
        if !browser_open {
            self.browser.take();
        }

        let mut background_open = true;
        if let Some(background) = self.background.as_ref() {
            egui::Window::new("Configure Background")
                .open(&mut background_open)
                .resizable(false)
                .show(ctx, |ui| {
                    background.render(ui);
                });
        }
        if !background_open {
            self.background.take();
        }

        let mut settings_open = true;
        if let Some(settings) = self.settings.as_mut() {
            egui::Window::new("Settings")
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    settings_open = settings.render(ui);
                });
        }
        if !settings_open {
            self.settings.take();
        }

        let mut performance_open = true;
        if let Some(performance) = self.performance.as_ref() {
            egui::Window::new("Configure Background")
                .open(&mut performance_open)
                .resizable(false)
                .show(ctx, |ui| {
                    performance.render(ui);
                });
        }
        if !performance_open {
            self.performance.take();
        }
    }

    pub fn open_window(&mut self, window: Windows, gfx: &Gfx) {
        match window {
            Windows::SceneBrowser => {
                if self.browser.is_none() {
                    self.browser = Self::browser(&self.app_state, gfx);
                }
            }
            Windows::SceneSettings => {
                if self.scene.is_none() {
                    if let Some(scene) = self.app_state.get().scene() {
                        self.scene = Self::scene_ui(scene);
                    }
                }
            }
            Windows::Settings => {
                if self.settings.is_none() {
                    self.settings = Self::settings(&self.app_state);
                }
            }
            Windows::Performance => {
                if self.performance.is_none() {
                    self.performance = Self::performance();
                }
            }
            Windows::ConfigureBackground => {
                if self.background.is_none() {
                    self.background = Self::background(&self.app_state);
                }
            }
        }
    }
}
