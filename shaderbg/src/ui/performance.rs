pub struct Performance {}

impl Performance {
    pub fn new() -> Performance {
        Performance {}
    }

    pub fn render(&self, ui: &mut egui::Ui) {
        ui.label("Not implemented");
    }
}
