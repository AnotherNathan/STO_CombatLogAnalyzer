use std::path::PathBuf;

use eframe::egui::*;
use rfd::FileDialog;
use serde::*;
use serde_json::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    pub combatlog_file: String,
}

#[derive(Default)]
pub struct SettingsWindow {
    is_open: bool,
    modified_settings: Settings,
    result: SettingsResult,
}

#[derive(Default, Clone, Copy)]
pub enum SettingsResult {
    #[default]
    NoChanges,
    ReloadLog,
}

impl SettingsWindow {
    pub fn show(&mut self, ctx: &Context, ui: &mut Ui, settings: &mut Settings) -> SettingsResult {
        self.result = SettingsResult::NoChanges;
        if ui.selectable_label(self.is_open, "Settings").clicked() && !self.is_open {
            self.is_open = true;
            self.modified_settings = settings.clone();
        }

        if self.is_open {
            Window::new("Settings").collapsible(false).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("combatlog file");
                    if ui.button("browse").clicked() {
                        // TODO find out how to set the parent
                        if let Some(new_combatlog_file) = FileDialog::new()
                            .add_filter("combatlog", &["log"])
                            .pick_file()
                        {
                            self.modified_settings.combatlog_file =
                                new_combatlog_file.display().to_string();
                        }
                    }
                });
                TextEdit::singleline(&mut self.modified_settings.combatlog_file)
                    .desired_width(f32::MAX)
                    .show(ui);

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Ok").clicked() {
                        if self.modified_settings.combatlog_file != settings.combatlog_file {
                            self.modified_settings.save();
                            self.result = SettingsResult::ReloadLog;
                        }

                        self.is_open = false;
                        *settings = self.modified_settings.clone();
                    }

                    if ui.button("Cancel").clicked() {
                        self.is_open = false;
                    }
                })
            });
        }
        self.result
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::file_path()
            .and_then(|f| std::fs::read_to_string(&f).ok())
            .map(|d| serde_json::from_str(&d).ok())
            .flatten()
            .unwrap_or_else(|| Self {
                combatlog_file: Default::default(),
            })
    }
}

impl Settings {
    fn file_path() -> Option<PathBuf> {
        let mut path = std::env::current_exe().ok()?;
        path.pop();
        path.push("settings.json");
        Some(path)
    }

    fn save(&self) {
        let file_path = match Self::file_path() {
            Some(p) => p,
            None => {
                return;
            }
        };
        let data = match serde_json::to_string(self) {
            Ok(d) => d,
            Err(_) => {
                return;
            }
        };

        let _ = std::fs::write(&file_path, data);
    }
}
