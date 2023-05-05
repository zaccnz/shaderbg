/*
 * UI for changing a scenes settings
 */

use std::{cell::RefCell, collections::HashMap};

use imgui::Ui;

use crate::{
    app::{AppEvent, AppState},
    io::scene::{
        setting::Setting as SettingDescriptor, util::DeserializableMap, Ui as UiDescriptor,
    },
    scene::Setting,
};

type Colour3State = (String, [f32; 3]);

pub struct SceneUi {
    app_state: AppState,
    setting_descriptors: DeserializableMap<SettingDescriptor>,
    ui_descriptors: Vec<UiDescriptor>,
    colour3_state: RefCell<HashMap<String, Colour3State>>,
}

impl SceneUi {
    pub fn new(app_state: AppState) -> SceneUi {
        let (setting_descriptors, ui_descriptors) = {
            let scene_descriptor = &app_state.get().scene.descriptor;
            (
                scene_descriptor.settings.clone(),
                scene_descriptor.ui.clone(),
            )
        };

        SceneUi {
            app_state,
            setting_descriptors,
            ui_descriptors,
            colour3_state: RefCell::new(HashMap::new()),
        }
    }

    fn render_float(
        ui: &Ui,
        key: &String,
        label: &String,
        mut value: f32,
        min: f32,
        max: f32,
        app_state: &AppState,
    ) {
        if ui.slider(label, min, max, &mut value) {
            app_state
                .send(AppEvent::SettingUpdated(key.clone(), Setting::Float(value)))
                .unwrap();
        }
    }

    fn render_colour3(
        ui: &Ui,
        key: &String,
        label: &String,
        mut value: [f32; 3],
        app_state: &AppState,
        colour3_state: &mut HashMap<String, Colour3State>,
    ) {
        let imgui_colour = [value[0], value[1], value[2], 1.0];
        let mut change = false;

        let popup_id = format!("picker::{}", key);
        let button_label_text = format!("Set {}", label);

        let button_label = if let Some((label, _)) = colour3_state.get(key) {
            label
        } else {
            &button_label_text
        };

        let mut open = false;

        open |= ui.color_button(label.as_str(), imgui_colour);
        ui.same_line_with_spacing(0.0, unsafe { ui.style() }.item_inner_spacing[0]);
        open |= ui.button(button_label.as_str());

        if open {
            ui.open_popup(popup_id.as_str());
            colour3_state.insert(key.clone(), (button_label_text, value.clone()));
        }

        if let Some(popup) = ui.begin_popup(popup_id.as_str()) {
            change |= ui.color_picker3(label, &mut value);

            if ui.button("Save") {
                ui.close_current_popup();
            }
            ui.same_line_with_spacing(0.0, unsafe { ui.style() }.item_inner_spacing[0]);
            if ui.button("Cancel") {
                ui.close_current_popup();
                value = if let Some((_, colour)) = colour3_state.get(key) {
                    *colour
                } else {
                    eprintln!("Colour3 initial value went missing!");
                    [0.0, 0.0, 0.0]
                };
                change = true;
                popup.end();
            }

            if change {
                app_state
                    .send(AppEvent::SettingUpdated(
                        key.clone(),
                        Setting::Colour3(value),
                    ))
                    .unwrap();
            }
        }
    }

    // pub fn on_scene_change(&mut self) {}

    pub fn render(&mut self, ui: &Ui) {
        for element in self.ui_descriptors.iter() {
            match element {
                UiDescriptor::Setting { setting: key } => {
                    if let Some(setting) = self.setting_descriptors.get(&key) {
                        match setting {
                            SettingDescriptor::Colour3 { label, .. } => {
                                let value = {
                                    let state = self.app_state.get();
                                    match state.scene.settings.get(key).unwrap() {
                                        Setting::Colour3(value) => *value,
                                        _ => panic!("Setting type mismatch in Ui"),
                                    }
                                };

                                SceneUi::render_colour3(
                                    ui,
                                    key,
                                    label,
                                    value,
                                    &self.app_state,
                                    self.colour3_state.get_mut(),
                                );
                            }
                            SettingDescriptor::Float {
                                label, min, max, ..
                            } => {
                                let value = {
                                    let state = self.app_state.get();
                                    match state.scene.settings.get(key).unwrap() {
                                        Setting::Float(value) => *value,
                                        _ => panic!("Setting type mismatch in Ui"),
                                    }
                                };

                                SceneUi::render_float(
                                    ui,
                                    key,
                                    label,
                                    value,
                                    *min,
                                    *max,
                                    &self.app_state,
                                );
                            }
                        }
                    } else {
                        ui.text(format!("Missing setting for key {}", key));
                    }
                }
                UiDescriptor::Separator => ui.separator(),
                UiDescriptor::Text { text } => ui.text(text),
            }
        }
        //for (key, setting_desc) in self.setting_descriptors.iter() {}
    }
}
