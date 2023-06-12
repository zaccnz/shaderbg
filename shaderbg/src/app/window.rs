use egui::RichText;
/*
 * Main window
 */
use tao::{
    dpi::{LogicalSize, PhysicalSize},
    event::{Event, WindowEvent as TaoWindowEvent},
    event_loop::{ControlFlow, EventLoopWindowTarget},
    keyboard::KeyCode,
    window::{Window as TaoWindow, WindowBuilder, WindowId},
};

use crate::{
    app::{AppState, WindowEvent},
    egui_tao,
};
use shaderbg_render::{
    gfx::{self, Gfx, GfxContext},
    scene::{Resources, Setting},
};

pub struct Window {
    window: TaoWindow,
    gfx: Gfx,
    #[allow(dead_code)]
    app_state: AppState,
    egui: egui_tao::State,
    resources: Resources,
    settings_open: bool,
    scene_ui: gfx::ui::Scene,
}

impl Window {
    pub fn build(event_loop: &EventLoopWindowTarget<WindowEvent>, app_state: AppState) -> Window {
        #[cfg(target_os = "macos")]
        {
            use tao::platform::macos::{ActivationPolicy, EventLoopWindowTargetExtMacOS};
            event_loop.set_activation_policy_at_runtime(ActivationPolicy::Regular);
        }

        let window = WindowBuilder::new()
            .with_title("shaderbg")
            .with_inner_size(LogicalSize::new(1024, 576))
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

        let resources = Resources::new(
            &app_state.get().scene,
            &gfx.device,
            &gfx.config,
            app_state.get().time,
        )
        .unwrap();

        let scene_ui = gfx::ui::Scene::new(&app_state.get().scene.descriptor);

        Window {
            window,
            gfx,
            app_state,
            egui: egui_platform,
            resources,
            settings_open: false,
            scene_ui,
        }
    }

    pub fn update_setting(&mut self, key: String, value: Setting) {
        self.resources.update_setting(key, value);
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
                    (state.scene.settings.clone(), state.time.clone())
                };

                let full_output = self.gfx.render(
                    Some(&mut self.resources),
                    time,
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
                                ui.heading(RichText::new("Current Scene").strong());
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(
                                            self.app_state.get().scene.descriptor.meta.name.clone(),
                                        )
                                        .strong(),
                                    );

                                    ui.label(format!(
                                        "({})",
                                        self.app_state.get().scene.descriptor.meta.version.clone()
                                    ));
                                });
                                if ui.button("Scene Settings").clicked() {
                                    self.settings_open = true;
                                    self.scene_ui.load_settings(&self.app_state.get().scene);
                                }
                            });

                        let mut open = self.settings_open;
                        egui::Window::new("Scene Settings")
                            .open(&mut open)
                            .resizable(false)
                            .show(ctx, |ui| {
                                self.settings_open =
                                    self.scene_ui.render(ui, settings, &mut changes);
                            });
                        self.settings_open &= open;
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

    pub fn will_close(&self, event_loop: &EventLoopWindowTarget<WindowEvent>) {
        #[cfg(target_os = "macos")]
        {
            use tao::platform::macos::{ActivationPolicy, EventLoopWindowTargetExtMacOS};
            event_loop.set_activation_policy_at_runtime(ActivationPolicy::Accessory);
        }
    }
}
