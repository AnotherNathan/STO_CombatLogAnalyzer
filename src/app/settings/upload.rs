use eframe::egui::Ui;

use super::Settings;

#[derive(Default)]
pub struct UploadTab {}

impl UploadTab {
    pub fn show(&mut self, modified_settings: &mut Settings, ui: &mut Ui) {
        ui.label("CLA can upload logs to the OSCR server (a different combat log parser) for comparing records with other players.");
        ui.add_space(20.0);
        ui.label("OSCR Upload URL:");
        ui.text_edit_singleline(&mut modified_settings.upload.oscr_url);
    }
}
