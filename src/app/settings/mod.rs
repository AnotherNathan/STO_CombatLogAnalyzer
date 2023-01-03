pub use app_settings::Settings;
use eframe::egui::*;

use self::{analysis::AnalysisTab, file::FileTab};

mod analysis;
mod app_settings;
mod file;

#[derive(Default)]
pub struct SettingsWindow {
    is_open: bool,
    modified_settings: Settings,
    result: SettingsResult,
    selected_tab: SettingsTab,
    file_tab: FileTab,
    analysis_tab: AnalysisTab,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    #[default]
    File,
    Analysis,
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
            self.file_tab.update_slider_displays(settings);
        }

        if self.is_open {
            Window::new("Settings")
                .collapsible(false)
                .default_size([800.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(self.selected_tab == SettingsTab::File, "file")
                            .clicked()
                        {
                            self.selected_tab = SettingsTab::File;
                        }

                        if ui
                            .selectable_label(
                                self.selected_tab == SettingsTab::Analysis,
                                "analysis",
                            )
                            .clicked()
                        {
                            self.selected_tab = SettingsTab::Analysis;
                        }
                    });

                    ui.separator();

                    match self.selected_tab {
                        SettingsTab::File => self.file_tab.show(&mut self.modified_settings, ui),
                        SettingsTab::Analysis => {
                            self.analysis_tab.show(&mut self.modified_settings, ui)
                        }
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Ok").clicked() {
                            if self.modified_settings != *settings {
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
