/*
 * Main window
 */
use tao::{
    dpi::{LogicalSize, PhysicalSize},
    event::{Event, WindowEvent as TaoWindowEvent},
    event_loop::EventLoopWindowTarget,
    keyboard::KeyCode,
    window::{Theme, Window as TaoWindow, WindowBuilder, WindowId},
};

use crate::{
    app::{AppEvent, AppState, MenuBuilder, WindowEvent},
    egui_tao,
    io::{TrayConfig, UiTheme},
    ui,
};
use shaderbg_render::{
    gfx::{self, buffer::ShaderToy, Gfx, GfxContext},
    scene::{Resources, Setting, Settings},
};

#[derive(Debug)]
pub enum Windows {
    SceneSettings,
    SceneBrowser,
    ConfigureBackground,
    Settings,
    Performance,
}

pub struct Window {
    window: TaoWindow,
    gfx: Gfx,
    #[allow(dead_code)]
    app_state: AppState,
    egui: egui_tao::State,
    settings: Option<Settings>,
    resources: Option<Resources>,
    shadertoy: ShaderToy,
    // ui
    scene_ui: Option<gfx::ui::Scene>,
    browser: Option<gfx::ui::Browser>,
    settings_ui: Option<ui::Settings>,
}

impl Window {
    pub fn build(
        event_loop: &EventLoopWindowTarget<WindowEvent>,
        app_state: AppState,
        menu_builder: &mut MenuBuilder,
    ) -> Window {
        #[cfg(target_os = "macos")]
        {
            use tao::platform::macos::{ActivationPolicy, EventLoopWindowTargetExtMacOS};
            event_loop.set_activation_policy_at_runtime(ActivationPolicy::Regular);
        }

        let window = WindowBuilder::new()
            .with_title("shaderbg")
            .with_inner_size(LogicalSize::new(1024, 576))
            .with_menu(menu_builder.build_window_menu())
            .build(&event_loop)
            .unwrap();

        #[cfg(target_os = "macos")]
        {
            window.set_focus();
        }

        let gfx_context = GfxContext::new(&window);

        let size = window.inner_size();
        let gfx = pollster::block_on(Gfx::new(gfx_context, size.width, size.height, true));

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

        gfx.ui.as_ref().unwrap().context().set_visuals(visuals);

        let mut egui_platform = egui_tao::State::new(&window);
        egui_platform.set_pixels_per_point(window.scale_factor() as f32);

        let shadertoy = ShaderToy::new();

        let (resources, settings) = if let Some(scene) = app_state.get().scene() {
            (
                Some(
                    Resources::new(
                        scene,
                        &gfx.device,
                        gfx.config.width,
                        gfx.config.height,
                        gfx.config.format,
                    )
                    .unwrap(),
                ),
                Some(scene.settings.clone()),
            )
        } else {
            (None, None)
        };

        Window {
            window,
            gfx,
            app_state,
            egui: egui_platform,
            settings,
            resources,
            scene_ui: None,
            browser: None,
            settings_ui: None,
            shadertoy,
        }
    }

    pub fn update_setting(&mut self, key: String, value: Setting) {
        if let Some(settings) = self.settings.as_mut() {
            settings.update(&key, value.clone());
        }

        if let Some(resources) = self.resources.as_mut() {
            resources.update_setting(key, value);
        }
    }

    pub fn get_window_id(&self) -> WindowId {
        self.window.id()
    }

    pub fn handle(&mut self, event: Event<WindowEvent>) -> bool {
        if let Event::WindowEvent { event, .. } = &event {
            if let Some(ui) = self.gfx.ui.as_ref() {
                let _ = self.egui.on_event(&ui.context, event);
            }
        }

        match event {
            Event::WindowEvent {
                event: TaoWindowEvent::CloseRequested,
                ..
            } => {
                let open_tray = {
                    let state = self.app_state.get();
                    state.config.tray_config == TrayConfig::CloseTo
                };
                if open_tray {
                    self.app_state
                        .send(AppEvent::Window(WindowEvent::StartTray))
                        .unwrap();
                }
                return false;
            }
            Event::WindowEvent {
                event: TaoWindowEvent::Resized(PhysicalSize { width, height }),
                ..
            } => {
                self.gfx.resized(width, height);
                if let Some(resources) = self.resources.as_mut() {
                    resources.resize(width, height);
                }
            }
            Event::WindowEvent {
                event:
                    TaoWindowEvent::ScaleFactorChanged {
                        new_inner_size: PhysicalSize { width, height },
                        scale_factor,
                    },
                ..
            } => {
                self.gfx.resized(*width, *height);
                if let Some(resources) = self.resources.as_mut() {
                    resources.resize(*width, *height);
                }
                self.egui.set_pixels_per_point(scale_factor as f32);
            }
            Event::WindowEvent {
                event:
                    TaoWindowEvent::KeyboardInput {
                        event:
                            tao::event::KeyEvent {
                                physical_key: KeyCode::Escape,
                                ..
                            },
                        ..
                    },
                ..
            } => {
                return false;
            }
            Event::WindowEvent {
                event: TaoWindowEvent::ThemeChanged(theme),
                ..
            } => {
                if self.app_state.get().config.theme == UiTheme::System {
                    match theme {
                        tao::window::Theme::Dark => {
                            if let Some(ui) = self.gfx.ui.as_ref() {
                                ui.context().set_visuals(egui::Visuals::dark());
                            }
                        }
                        tao::window::Theme::Light => {
                            if let Some(ui) = self.gfx.ui.as_ref() {
                                ui.context().set_visuals(egui::Visuals::light());
                            }
                        }
                        _ => (),
                    };
                }
            }
            Event::MainEventsCleared => self.window.request_redraw(),
            Event::RedrawEventsCleared => {
                let mut changes = Vec::new();
                let time = { self.app_state.get_time().clone() };

                let size = self.window.inner_size();

                self.shadertoy
                    .update(time.time, time.dt as f64, size.width, size.height);

                if let Some(browser) = self.browser.as_mut() {
                    browser.update_previews(
                        self.gfx.ui.as_mut().unwrap().renderer_mut(),
                        &self.gfx.queue,
                        &mut self.gfx.device,
                        time,
                    );
                }

                let mut open_browser = false;

                let full_output = self.gfx.render(
                    self.resources.as_mut(),
                    time,
                    self.shadertoy,
                    Some((
                        self.egui.pixels_per_point(),
                        self.egui.take_egui_input(&self.window),
                    )),
                    |ctx| {
                        egui::Window::new(if let Some(scene) = self.app_state.get().scene() {
                            format!("Menu - {}", scene.descriptor.meta.name)
                        } else {
                            "Menu".to_string()
                        })
                        .movable(false)
                        .resizable(false)
                        .id("shaderbg".into())
                        .show(ctx, |ui: &mut egui::Ui| {
                            if let Some(scene) = self.app_state.get().scene() {
                                ui.label("Scene");
                                if ui.button("Pause").clicked() {}
                                if ui.button("Reload").clicked() {}
                                if ui.button("Scene Settings").clicked() {
                                    self.scene_ui = Some(gfx::ui::Scene::new(
                                        &scene.descriptor,
                                        &scene.settings,
                                    ));
                                }
                            } else {
                                ui.heading("No Scene Loaded");
                            }
                            ui.label("App");
                            if ui.button("Scene Browser").clicked() {
                                open_browser = true;
                            }
                            if ui.button("Configure Background").clicked() {}
                            if ui.button("Settings").clicked() {
                                self.settings_ui = Some(ui::Settings::new(self.app_state.clone()));
                            }
                            if ui.button("Performance").clicked() {}
                        });

                        let settings = self.settings.as_ref();

                        let mut scene_ui_open = true;

                        if let Some(scene_ui) = self.scene_ui.as_mut() {
                            let mut open = true;
                            egui::Window::new("Scene Settings")
                                .open(&mut scene_ui_open)
                                .resizable(false)
                                .show(ctx, |ui| {
                                    if let Some(settings) = settings {
                                        open = scene_ui.render(ui, settings, &mut changes);
                                    } else {
                                        ui.heading("An error occurred");
                                    }
                                });
                            scene_ui_open &= open;
                        }

                        if !scene_ui_open {
                            self.scene_ui.take();
                        }

                        let mut browser_open = true;
                        let mut browser_reload = false;
                        if let Some(browser) = self.browser.as_ref() {
                            egui::Window::new("Scene Browser")
                                .open(&mut browser_open)
                                .resizable(false)
                                .collapsible(false)
                                .show(ctx, |ui| {
                                    let scene = browser.render(
                                        ui,
                                        self.app_state.get().current_scene(),
                                        Some(&mut browser_reload),
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
                        if browser_reload {
                            println!("reload scenes");
                        }

                        let mut settings_open = true;

                        if let Some(settings) = self.settings_ui.as_mut() {
                            egui::Window::new("Settings")
                                .resizable(false)
                                .collapsible(false)
                                .show(ctx, |ui| {
                                    settings_open = settings.render(ui);
                                });
                        }

                        if !settings_open {
                            self.settings_ui.take();
                        }
                    },
                );

                for (key, value) in changes {
                    self.app_state
                        .send(super::AppEvent::SettingUpdated(key, value))
                        .unwrap();
                }

                if open_browser {
                    self.browser = Some(gfx::ui::Browser::new(
                        self.app_state
                            .get()
                            .scenes
                            .iter()
                            .map(|entry| (entry.name.clone().to_string(), &entry.scene))
                            .collect(),
                        &self.gfx.device,
                    ));
                }

                if let Some(full_output) = full_output {
                    if let Some(ui) = self.gfx.ui.as_ref() {
                        self.egui.handle_platform_output(
                            &self.window,
                            &ui.context,
                            full_output.platform_output.clone(),
                        );
                    }
                }
            }
            _ => (),
        }

        true
    }

    pub fn open_ui_window(&mut self, window: Windows) {
        match window {
            Windows::SceneBrowser => {
                if self.browser.is_none() {
                    self.browser = Some(gfx::ui::Browser::new(
                        self.app_state
                            .get()
                            .scenes
                            .iter()
                            .map(|entry| (entry.name.clone().to_string(), &entry.scene))
                            .collect(),
                        &self.gfx.device,
                    ));
                }
            }
            Windows::SceneSettings => {
                if self.scene_ui.is_none() {
                    if let Some(scene) = self.app_state.get().scene() {
                        self.scene_ui =
                            Some(gfx::ui::Scene::new(&scene.descriptor, &scene.settings));
                    }
                }
            }
            Windows::Settings => {
                if self.settings_ui.is_none() {
                    self.settings_ui = Some(ui::Settings::new(self.app_state.clone()));
                }
            }
            Windows::Performance => todo!(),
            Windows::ConfigureBackground => todo!(),
        }
    }

    pub fn rebuild_menus(&mut self, menu_builder: &mut MenuBuilder) {
        let menu = menu_builder.build_window_menu();
        self.window.set_menu(Some(menu));
    }

    pub fn scene_changed(&mut self) {
        if let Some(scene) = self.app_state.get().scene() {
            self.resources = Some(
                Resources::new(
                    scene,
                    &self.gfx.device,
                    self.gfx.config.width,
                    self.gfx.config.height,
                    self.gfx.config.format,
                )
                .unwrap(),
            );

            self.settings = Some(scene.settings.clone());
            if self.scene_ui.is_some() {
                self.scene_ui = Some(gfx::ui::Scene::new(&scene.descriptor, &scene.settings));
            }
        } else {
            self.resources = None;
            self.settings = None;
        }
    }

    pub fn update_theme(&mut self, theme: UiTheme) {
        let visuals = match theme {
            UiTheme::Dark => egui::Visuals::dark(),
            UiTheme::Light => egui::Visuals::light(),
            UiTheme::System => {
                if self.window.theme() == Theme::Dark {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                }
            }
        };

        self.gfx.ui.as_ref().unwrap().context().set_visuals(visuals);
    }

    pub fn will_close(&self, event_loop: &EventLoopWindowTarget<WindowEvent>) {
        #[cfg(target_os = "macos")]
        {
            use tao::platform::macos::{ActivationPolicy, EventLoopWindowTargetExtMacOS};
            event_loop.set_activation_policy_at_runtime(ActivationPolicy::Accessory);
        }
    }
}
