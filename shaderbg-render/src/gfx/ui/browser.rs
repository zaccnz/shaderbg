use std::collections::HashMap;

use crate::scene::{io::Metadata, Resources, Scene};

pub struct Browser {
    scenes: Box<[(String, Metadata)]>,
    resources: HashMap<String, Resources>,
}

impl Browser {
    pub fn new(scenes: Vec<(String, &Scene)>) -> Browser {
        Browser {
            scenes: scenes
                .iter()
                .map(|(name, scene)| (name.clone(), scene.descriptor.meta.clone()))
                .collect(),
            resources: HashMap::new(),
        }
    }

    pub fn render(
        &self,
        ui: &mut egui::Ui,
        current_scene: Option<usize>,
        reload: Option<&mut bool>,
    ) -> Option<usize> {
        let mut selected = None;
        ui.horizontal(|ui| {
            ui.heading(format!("{} scenes loaded", self.scenes.len()));
            if let Some(reload) = reload {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Reload").clicked() {
                        *reload = true;
                    }
                });
            }
        });
        ui.separator();
        for (index, (_, meta)) in self.scenes.iter().enumerate() {
            ui.label(format!("{} ({})", meta.name, meta.version));
            if Some(index) == current_scene {
                ui.label("selected");
            }
            if ui.button("Select").clicked() {
                selected = Some(index)
            }
        }
        selected
    }
}
