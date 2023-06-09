/*
 * UI for changing a scenes settings
 */
use std::collections::HashMap;

use egui::RichText;

use crate::scene::{
    io::{
        setting::Setting as SettingDescriptor, setting::SettingValue, util::DeserializableMap,
        Descriptor, Ui as UiDescriptor,
    },
    Settings,
};

enum SceneUiElement {
    Separator,
    Text(String),
    SettingGroup(Vec<String>),
}

#[derive(PartialEq)]
pub enum SceneUiResult {
    Open,
    Closed,
    Saved,
}

pub struct Scene {
    setting_descriptors: DeserializableMap<SettingDescriptor>,
    elements: Vec<SceneUiElement>,
    original_values: HashMap<String, SettingValue>,
}

impl Scene {
    pub fn new(descriptor: &Descriptor, settings: &Settings) -> Scene {
        let mut elements = Vec::new();

        for ui_desc in descriptor.ui.iter() {
            match ui_desc {
                UiDescriptor::Separator => elements.push(SceneUiElement::Separator),
                UiDescriptor::Text { text } => elements.push(SceneUiElement::Text(text.clone())),
                UiDescriptor::Setting { setting } => match elements.last_mut() {
                    Some(SceneUiElement::SettingGroup(vec)) => {
                        vec.push(setting.clone());
                    }
                    _ => elements.push(SceneUiElement::SettingGroup(vec![setting.clone()])),
                },
            }
        }

        let mut original_values = HashMap::new();

        for (key, value) in settings.iter() {
            original_values.insert(key.clone(), value.clone());
        }

        Scene {
            setting_descriptors: descriptor.settings.clone(),
            elements,
            original_values,
        }
    }

    fn render_float(
        ui: &mut egui::Ui,
        label: &String,
        mut value: f32,
        min: f32,
        max: f32,
    ) -> Option<SettingValue> {
        let mut change = None;
        ui.label(label);
        ui.spacing_mut().slider_width = 220.0;
        if ui.add(egui::Slider::new(&mut value, min..=max)).changed() {
            change = Some(SettingValue::Float(value))
        }
        ui.end_row();
        change
    }

    fn render_colour3(
        ui: &mut egui::Ui,
        label: &String,
        mut value: [f32; 3],
    ) -> Option<SettingValue> {
        let mut change = None;
        ui.label(label);
        if ui.color_edit_button_rgb(&mut value).changed() {
            change = Some(SettingValue::Colour3(value));
        }
        ui.end_row();

        change
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        scene_settings: &Settings,
        changes: &mut Vec<(String, SettingValue)>,
    ) -> SceneUiResult {
        let mut result = SceneUiResult::Open;

        let mut group_count = 0;

        for element in self.elements.iter() {
            match element {
                SceneUiElement::SettingGroup(settings) => {
                    egui::Grid::new(format!("settings_group_{}", group_count)).show(ui, |ui| {
                        for key in settings {
                            if let Some(setting) = self.setting_descriptors.get(key) {
                                let change = match setting {
                                    SettingDescriptor::Colour3 { label, .. } => {
                                        let value = {
                                            match scene_settings.get(key).unwrap() {
                                                SettingValue::Colour3(value) => {
                                                    [value[0], value[1], value[2]]
                                                }
                                                _ => panic!("Setting type mismatch in Ui"),
                                            }
                                        };

                                        Scene::render_colour3(ui, label, value)
                                    }
                                    SettingDescriptor::Float {
                                        label, min, max, ..
                                    } => {
                                        let value = {
                                            match scene_settings.get(key).unwrap() {
                                                SettingValue::Float(value) => *value,
                                                _ => panic!("Setting type mismatch in Ui"),
                                            }
                                        };

                                        Scene::render_float(ui, label, value, *min, *max)
                                    }
                                };

                                if let Some(change) = change {
                                    changes.push((key.clone(), change));
                                }
                            } else {
                                ui.label(format!("Missing setting for key {}", key));
                            }
                        }
                    });
                    group_count += 1;
                }
                SceneUiElement::Separator => drop(ui.separator()),
                SceneUiElement::Text(text) => drop(ui.label(RichText::new(text).strong())),
            }
        }

        ui.horizontal(|ui| {
            if ui.button("Reset").clicked() {
                changes.clear();
                for (key, value) in self.setting_descriptors.iter() {
                    changes.push((key.clone(), SettingValue::from_descriptor(value).unwrap()));
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Save").clicked() {
                    result = SceneUiResult::Saved;
                };

                if ui.button("Cancel").clicked() {
                    result = SceneUiResult::Closed;
                    changes.clear();
                    for (key, value) in self.original_values.iter() {
                        changes.push((key.clone(), value.clone()));
                    }
                };
            });
        });

        result
    }
}
