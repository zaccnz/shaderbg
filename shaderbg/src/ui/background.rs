use crate::app::{AppEvent, AppState, ThreadEvent};

pub struct Background {
    app_state: AppState,
}

impl Background {
    pub fn new(app_state: AppState) -> Background {
        Background { app_state }
    }

    pub fn render(&self, ui: &mut egui::Ui) {
        let mut background_enabled = self.app_state.get().background_open;

        if ui.checkbox(&mut background_enabled, "Enabled").changed() {
            self.app_state
                .send(if background_enabled {
                    AppEvent::Window(ThreadEvent::StartBackground)
                } else {
                    AppEvent::BackgroundClosed(true)
                })
                .unwrap()
        }
    }
}
