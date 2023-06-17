/*
 * Main window
 */
use egui::RichText;
use tao::{
    dpi::{LogicalSize, PhysicalSize},
    event::{Event, WindowEvent as TaoWindowEvent},
    event_loop::{ControlFlow, EventLoopWindowTarget},
    keyboard::KeyCode,
    window::{Window as TaoWindow, WindowBuilder, WindowId},
};

use crate::{
    app::{AppState, MenuBuilder, WindowEvent},
    egui_tao,
};
use shaderbg_render::{
    gfx::{self, buffer::ShaderToy, Gfx, GfxContext},
    scene::{Resources, Setting},
};

#[derive(Debug)]
pub enum Windows {
    SceneBrowser,
    SceneSettings,
    Settings,
    Performance,
}

pub struct Window {
    window: TaoWindow,
    gfx: Gfx,
    #[allow(dead_code)]
    app_state: AppState,
    egui: egui_tao::State,
    resources: Option<Resources>,
    scene_ui: Option<gfx::ui::Scene>,
    shadertoy: ShaderToy,
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

        let mut egui_platform = egui_tao::State::new(&window);
        egui_platform.set_pixels_per_point(window.scale_factor() as f32);

        let shadertoy = ShaderToy::new();

        let resources = if let Some(scene) = app_state.get().scene() {
            Some(
                Resources::new(
                    scene,
                    &gfx.device,
                    &gfx.config,
                    app_state.get().time,
                    shadertoy,
                )
                .unwrap(),
            )
        } else {
            None
        };

        Window {
            window,
            gfx,
            app_state,
            egui: egui_platform,
            resources,
            scene_ui: None,
            shadertoy,
        }
    }

    pub fn update_setting(&mut self, key: String, value: Setting) {
        if let Some(resources) = self.resources.as_mut() {
            resources.update_setting(key, value);
        }
    }

    pub fn get_window_id(&self) -> WindowId {
        self.window.id()
    }

    pub fn handle(&mut self, event: Event<WindowEvent>, _control_flow: &mut ControlFlow) -> bool {
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
                return false;
            }
            Event::WindowEvent {
                event: TaoWindowEvent::Resized(PhysicalSize { width, height }),
                ..
            } => {
                self.gfx.resized(width, height);
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
            Event::MainEventsCleared => self.window.request_redraw(),
            Event::RedrawEventsCleared => {
                let mut changes = Vec::new();
                let (settings, time) = {
                    let state = self.app_state.get();
                    (
                        state.scene().map(|scene| scene.settings.clone()),
                        state.time.clone(),
                    )
                };

                let size = self.window.inner_size();

                self.shadertoy
                    .update(time.time, time.dt as f64, size.width, size.height);

                let full_output = self.gfx.render(
                    self.resources.as_mut(),
                    time,
                    self.shadertoy,
                    Some((
                        self.egui.pixels_per_point(),
                        self.egui.take_egui_input(&self.window),
                    )),
                    |ctx| {
                        egui::Window::new("shaderbg")
                            .movable(false)
                            .resizable(false)
                            .title_bar(false)
                            .show(ctx, |ui| {
                                if let Some(scene) = self.app_state.get().scene() {
                                    ui.heading(RichText::new("Current Scene").strong());
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(scene.descriptor.meta.name.clone())
                                                .strong(),
                                        );

                                        ui.label(format!(
                                            "({})",
                                            scene.descriptor.meta.version.clone()
                                        ));
                                    });
                                    if ui.button("Scene Settings").clicked() {
                                        self.scene_ui = Some(gfx::ui::Scene::new(
                                            &scene.descriptor,
                                            &scene.settings,
                                        ));
                                    }
                                } else {
                                    ui.heading("No Scene Loaded");
                                }
                            });

                        let mut open = true;
                        let mut win_open = true;

                        if let Some(scene_ui) = self.scene_ui.as_mut() {
                            egui::Window::new("Scene Settings")
                                .open(&mut win_open)
                                .resizable(false)
                                .show(ctx, |ui| {
                                    if let Some(settings) = settings {
                                        open = scene_ui.render(ui, settings, &mut changes);
                                    } else {
                                        ui.heading("An error occurred");
                                    }
                                });
                        }
                        if !open || !win_open {
                            self.scene_ui.take();
                        }
                    },
                );

                for (key, value) in changes {
                    self.app_state
                        .send(super::AppEvent::SettingUpdated(key, value))
                        .unwrap();
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
            Windows::SceneBrowser => todo!(),
            Windows::SceneSettings => {
                if let Some(scene) = self.app_state.get().scene() {
                    self.scene_ui = Some(gfx::ui::Scene::new(&scene.descriptor, &scene.settings));
                }
            }
            Windows::Settings => todo!(),
            Windows::Performance => todo!(),
        }
    }

    pub fn rebuild_menus(&mut self, menu_builder: &mut MenuBuilder) {
        let menu = menu_builder.build_window_menu();
        self.window.set_menu(Some(menu));
    }

    pub fn scene_changed(&mut self) {
        self.resources = if let Some(scene) = self.app_state.get().scene() {
            Some(
                Resources::new(
                    scene,
                    &self.gfx.device,
                    &self.gfx.config,
                    self.app_state.get().time,
                    self.shadertoy,
                )
                .unwrap(),
            )
        } else {
            None
        };
    }

    pub fn will_close(&self, event_loop: &EventLoopWindowTarget<WindowEvent>) {
        #[cfg(target_os = "macos")]
        {
            use tao::platform::macos::{ActivationPolicy, EventLoopWindowTargetExtMacOS};
            event_loop.set_activation_policy_at_runtime(ActivationPolicy::Accessory);
        }
    }
}
