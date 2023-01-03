use eframe::egui::*;
use rfd::FileDialog;
use std::fmt::Write;

use super::Settings;

#[derive(Default)]
pub struct FileTab {
    combat_separation_time: String,
    auto_refresh_interval: String,
}

impl FileTab {
    pub fn show(&mut self, modified_settings: &mut Settings, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("combatlog file");
            if ui.button("browse").clicked() {
                // TODO find out how to set the parent
                if let Some(new_combatlog_file) = FileDialog::new()
                    .add_filter("combatlog", &["log"])
                    .pick_file()
                {
                    modified_settings.analysis.combatlog_file =
                        new_combatlog_file.display().to_string();
                }
            }
        });
        TextEdit::singleline(&mut modified_settings.analysis.combatlog_file)
            .desired_width(f32::MAX)
            .show(ui);

        ui.separator();

        ui.label("combat separation time in seconds");
        ui.horizontal(|ui| {
            if Slider::new(
                &mut modified_settings.analysis.combat_separation_time_seconds,
                15.0..=240.0,
            )
            .clamp_to_range(false)
            .show_value(false)
            .step_by(15.0)
            .ui(ui)
            .changed()
            {
                self.update_combat_separation_time_display(modified_settings);
            }

            if TextEdit::singleline(&mut self.combat_separation_time)
                .desired_width(40.0)
                .show(ui)
                .response
                .changed()
            {
                if let Ok(combat_separation_time) = self.combat_separation_time.parse::<f64>() {
                    modified_settings.analysis.combat_separation_time_seconds =
                        combat_separation_time.max(0.0);
                }
            }
        });

        ui.separator();

        ui.checkbox(
            &mut modified_settings.auto_refresh.enable,
            "auto refresh when log changes",
        );
        ui.label("auto refresh interval in seconds");
        ui.horizontal(|ui| {
            if Slider::new(
                &mut modified_settings.auto_refresh.interval_seconds,
                1.0..=10.0,
            )
            .clamp_to_range(false)
            .show_value(false)
            .step_by(1.0)
            .ui(ui)
            .changed()
            {
                self.update_auto_refresh_interval_display(modified_settings);
            }

            if TextEdit::singleline(&mut self.auto_refresh_interval)
                .desired_width(40.0)
                .show(ui)
                .response
                .changed()
            {
                if let Ok(auto_refresh_interval) = self.auto_refresh_interval.parse::<f64>() {
                    modified_settings.auto_refresh.interval_seconds =
                        auto_refresh_interval.max(0.0);
                }
            }
        });

        ui.add_space(100.0);
    }

    pub fn update_slider_displays(&mut self, settings: &Settings) {
        self.update_combat_separation_time_display(settings);
        self.update_auto_refresh_interval_display(settings);
    }

    fn update_combat_separation_time_display(&mut self, settings: &Settings) {
        Self::update_slider_display(
            &mut self.combat_separation_time,
            settings.analysis.combat_separation_time_seconds,
        );
    }

    fn update_auto_refresh_interval_display(&mut self, settings: &Settings) {
        Self::update_slider_display(
            &mut self.auto_refresh_interval,
            settings.auto_refresh.interval_seconds,
        );
    }

    fn update_slider_display(display: &mut String, value: f64) {
        display.clear();
        write!(display, "{}", value).unwrap();
    }
}
