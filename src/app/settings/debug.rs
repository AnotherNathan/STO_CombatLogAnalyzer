use eframe::egui::{ComboBox, Ui};

use super::Settings;

#[derive(Default)]
pub struct DebugTab {}

impl DebugTab {
    pub fn show(&mut self, modified_settings: &mut Settings, ui: &mut Ui) {
        ui.label("App Log Settings");
        ui.label(
            "Any change to these settings requires a restart of the application to take affect.",
        );
        ui.checkbox(&mut modified_settings.debug.enable_log, "Enable Log");
        ComboBox::from_label("Log Filter Level")
            .selected_text(modified_settings.debug.log_level_filter.as_str())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut modified_settings.debug.log_level_filter,
                    log::LevelFilter::Info,
                    log::LevelFilter::Info.as_str(),
                );
                ui.selectable_value(
                    &mut modified_settings.debug.log_level_filter,
                    log::LevelFilter::Error,
                    log::LevelFilter::Error.as_str(),
                );
                ui.selectable_value(
                    &mut modified_settings.debug.log_level_filter,
                    log::LevelFilter::Warn,
                    log::LevelFilter::Warn.as_str(),
                );
                ui.selectable_value(
                    &mut modified_settings.debug.log_level_filter,
                    log::LevelFilter::Debug,
                    log::LevelFilter::Debug.as_str(),
                );
                ui.selectable_value(
                    &mut modified_settings.debug.log_level_filter,
                    log::LevelFilter::Trace,
                    log::LevelFilter::Trace.as_str(),
                );
            });
    }
}
