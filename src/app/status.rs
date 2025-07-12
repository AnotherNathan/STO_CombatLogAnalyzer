use eframe::egui::*;

use crate::helpers::number_formatting::NumberFormatter;

pub struct StatusIndicator {
    pub status: Status,
}

pub enum Status {
    NothingLoaded,
    Busy,
    LoadError {
        combatlog_file: String,
    },
    Loaded {
        combatlog_file: String,
        file_size: Option<u64>,
    },
}

impl StatusIndicator {
    pub fn new() -> Self {
        Self {
            status: Status::NothingLoaded,
        }
    }

    pub fn show(&mut self, is_analysis_busy: bool, ui: &mut Ui) {
        let status = if is_analysis_busy {
            &Status::Busy
        } else {
            &self.status
        };
        match status {
            Status::NothingLoaded => {
                ui.label(WidgetText::from("？").color(Color32::YELLOW))
                    .on_hover_text("nothing loaded yet");
            }
            Status::Busy => {
                ui.label(WidgetText::from("⏳").color(Color32::YELLOW))
                    .on_hover_text("Working..");
            }
            Status::LoadError {
                combatlog_file: path,
            } => {
                ui.label(WidgetText::from("✖").color(Color32::RED))
                    .on_hover_ui(|ui| {
                        ui.label("failed to load log from:");
                        ui.label(path);
                    });
            }
            Status::Loaded {
                combatlog_file,
                file_size,
            } => {
                ui.label(WidgetText::from("✔").color(Color32::GREEN))
                    .on_hover_ui(|ui| {
                        ui.label("log loaded from:");
                        ui.label(combatlog_file);

                        if let Some(file_size) = *file_size {
                            ui.add_space(20.0);
                            let size_text = format!(
                                "{}B",
                                NumberFormatter::new()
                                    .format_with_automated_suffixes(file_size as _)
                            );
                            ui.label("log file size:");
                            ui.label(size_text);
                        }
                    });
            }
        }
    }
}
