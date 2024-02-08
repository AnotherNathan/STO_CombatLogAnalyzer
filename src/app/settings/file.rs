use eframe::egui::*;
use eframe::Frame;
use rfd::FileDialog;

use crate::{
    app::analysis_handling::AnalysisHandler, custom_widgets::slider_text_edit::SliderTextEdit,
};

use super::Settings;

#[derive(Default)]
pub struct FileTab {
    clear_log_dialog: ClearLogDialog,
}

#[derive(Default)]
pub struct ClearLogDialog {
    is_open: bool,
}

impl FileTab {
    pub fn show(
        &mut self,
        analysis_handler: &AnalysisHandler,
        modified_settings: &mut Settings,
        ui: &mut Ui,
        frame: &Frame,
    ) {
        ui.horizontal(|ui| {
            ui.label("Combatlog File");
            if ui.button("Browse").clicked() {
                if let Some(new_combatlog_file) = FileDialog::new()
                    .set_title("Choose combatlog File")
                    .add_filter("combatlog", &["log"])
                    .set_parent(frame)
                    .pick_file()
                {
                    modified_settings.analysis.combatlog_file =
                        new_combatlog_file.display().to_string();
                }
            }

            self.clear_log_dialog.show(analysis_handler, ui);
        });
        TextEdit::singleline(&mut modified_settings.analysis.combatlog_file)
            .desired_width(f32::MAX)
            .show(ui);

        ui.separator();

        ui.label("Combat Separation Time in seconds");
        SliderTextEdit::new(
            &mut modified_settings.analysis.combat_separation_time_seconds,
            15.0..=240.0,
            "combat separation time slider",
        )
        .clamp_to_range(false)
        .step_by(15.0)
        .desired_text_edit_width(40.0)
        .clamp_min(1.0)
        .show(ui);

        ui.separator();

        ui.checkbox(
            &mut modified_settings.auto_refresh.enable,
            "Auto Refresh when log changes",
        );
        ui.label("Auto Refresh Interval in seconds");
        SliderTextEdit::new(
            &mut modified_settings.auto_refresh.interval_seconds,
            1.0..=10.0,
            "auto refresh interval slider",
        )
        .clamp_to_range(false)
        .step_by(1.0)
        .desired_text_edit_width(40.0)
        .clamp_min(0.1)
        .show(ui);
    }

    pub fn show_clear_log_dialog(&mut self, analysis_handler: &AnalysisHandler, ui: &mut Ui) {
        self.clear_log_dialog.show(analysis_handler, ui);
    }

    pub fn initialize(&mut self) {
        self.clear_log_dialog.initialize();
    }
}

impl ClearLogDialog {
    fn show(&mut self, analysis_handler: &AnalysisHandler, ui: &mut Ui) {
        let clear_response = ui.button("Clear Log File");

        let mut newly_opened = false;
        if clear_response.clicked() {
            self.is_open = true;
            newly_opened = true;
        }

        if !self.is_open {
            return;
        }

        let mut window = Window::new("Clear Log File")
            .collapsible(false)
            .default_size([400.0, 400.0])
            .resizable(false);
        if newly_opened {
            window = window.current_pos(clear_response.rect.min);
        }

        window.show(ui.ctx(), |ui| {
                ui.label("Clearing the log will delete all combats from log file except for the newest one.");
                ui.label("Note that for this to work properly all data from the log must have been analyzed.");
                ui.label("Make sure you refreshed before proceeding.");
                ui.add_space(20.0);
                ui.label("Do you wish to proceed?");

                ui.horizontal(|ui| {
                    if ui.button("Clear Log").clicked() {
                        self.is_open = false;
                        analysis_handler.clear_log()
                    }

                    if ui.button("Cancel").clicked() {
                        self.is_open = false;
                    }
                });
            });
    }

    fn initialize(&mut self) {
        self.is_open = false;
    }
}
