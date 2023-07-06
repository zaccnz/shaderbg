/*
 * Main window
 */
use tao::{
    dpi::{LogicalSize, PhysicalSize},
    event::{Event, WindowEvent},
    event_loop::EventLoopWindowTarget,
    keyboard::KeyCode,
    window::{Theme, Window as TaoWindow, WindowBuilder, WindowId},
};

use crate::{
    app::{AppEvent, AppState, MenuBuilder, ThreadEvent},
    io::{TrayConfig, UiTheme},
    ui::AppUi,
};
use shaderbg_render::{
    gfx::{buffer::ShaderToy, ui::SceneUiResult, Gfx, GfxContext},
    scene::{io::setting::SettingValue, Resources, Settings},
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
    app_state: AppState,
    settings: Option<Settings>,
    resources: Option<Resources>,
    shadertoy: ShaderToy,
    app_ui: AppUi,
}

impl Window {
    pub fn build(
        event_loop: &EventLoopWindowTarget<ThreadEvent>,
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
            .build(event_loop)
            .unwrap();

        #[cfg(target_os = "macos")]
        {
            window.set_focus();
        }

        let gfx_context = GfxContext::new(&window);

        let size = window.inner_size();
        let gfx = pollster::block_on(Gfx::new(gfx_context, size.width, size.height, true));

        let app_ui = AppUi::new(gfx.ui.as_ref().unwrap(), &window, app_state.clone());

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
            // egui: egui_platform,
            settings,
            resources,
            // scene_ui: None,
            // browser: None,
            // settings_ui: None,
            shadertoy,
            app_ui,
        }
    }

    pub fn update_setting(&mut self, key: String, value: SettingValue) {
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

    pub fn handle(&mut self, event: Event<ThreadEvent>) -> bool {
        if let Some(ui) = self.gfx.ui.as_ref() {
            self.app_ui.handle_event(&event, &ui.context);
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                let open_tray = {
                    let state = self.app_state.get();
                    state.config.tray_config == TrayConfig::CloseTo
                };
                if open_tray {
                    self.app_state
                        .send(AppEvent::Window(ThreadEvent::StartTray))
                        .unwrap();
                }
                return false;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(PhysicalSize { width, height }),
                ..
            } => {
                self.gfx.resized(width, height);
                if let Some(resources) = self.resources.as_mut() {
                    resources.resize(width, height);
                }
            }
            Event::WindowEvent {
                event:
                    WindowEvent::ScaleFactorChanged {
                        new_inner_size: PhysicalSize { width, height },
                        ..
                    },
                ..
            } => {
                self.gfx.resized(*width, *height);
                if let Some(resources) = self.resources.as_mut() {
                    resources.resize(*width, *height);
                }
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
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
                event: WindowEvent::ThemeChanged(theme),
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
                let time = { *self.app_state.get_time() };

                let size = self.window.inner_size();

                self.shadertoy
                    .update(time.time, time.dt as f64, size.width, size.height);

                self.app_ui.update_browser(&mut self.gfx, time);

                let mut reload_browser = false;
                let mut scene_ui_result = SceneUiResult::Open;

                let full_output = self.gfx.render(
                    self.resources.as_mut(),
                    time,
                    self.shadertoy,
                    Some(self.app_ui.get_input(&self.window)),
                    |ctx, gfx| {
                        self.app_ui.render(
                            ctx,
                            gfx,
                            self.settings.as_ref(),
                            &mut changes,
                            &mut scene_ui_result,
                            &mut reload_browser,
                        );
                    },
                );

                for (key, value) in changes {
                    self.app_state
                        .send(super::AppEvent::SettingUpdated(key, value))
                        .unwrap();
                }

                if let SceneUiResult::Saved = scene_ui_result {
                    self.app_state.send(AppEvent::SceneSettingsSaved).unwrap();
                }

                if let Some(full_output) = full_output {
                    self.app_ui.handle_full_output(
                        full_output.platform_output,
                        &self.window,
                        self.gfx.ui.as_ref().unwrap().context(),
                    );
                }
            }
            _ => (),
        }

        true
    }

    pub fn open_ui_window(&mut self, window: Windows) {
        self.app_ui.open_window(window, &self.gfx);
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
            self.app_ui.update_scene_ui(scene);
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

    #[allow(unused)]
    pub fn will_close(&self, event_loop: &EventLoopWindowTarget<ThreadEvent>) {
        #[cfg(target_os = "macos")]
        {
            use tao::platform::macos::{ActivationPolicy, EventLoopWindowTargetExtMacOS};
            event_loop.set_activation_policy_at_runtime(ActivationPolicy::Accessory);
        }
    }
}
