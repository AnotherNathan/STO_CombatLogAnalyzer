use std::ffi::OsStr;

pub use app_settings::Settings;
use eframe::{egui::*, Frame};

use crate::analyzer::Combat;

use self::{analysis::AnalysisTab, debug::DebugTab, file::FileTab, visuals::VisualsTab};

use super::{analysis_handling::AnalysisHandler, state::AppState};

mod analysis;
mod app_settings;
mod debug;
mod file;
mod visuals;

pub struct SettingsWindow {
    is_open: bool,
    modified_settings: Settings,
    selected_tab: SettingsTab,
    file_tab: FileTab,
    analysis_tab: AnalysisTab,
    visuals_tab: VisualsTab,
    debug_tab: DebugTab,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    #[default]
    File,
    Analysis,
    Visuals,
    Debug,
}

impl SettingsWindow {
    pub fn new(ctx: &Context, native_pixels_per_point: Option<f32>) -> Self {
        let mut visuals_tab = VisualsTab::default();
        let settings = Settings::load_or_default();
        visuals_tab.update_visuals(ctx, native_pixels_per_point, &settings);
        Self {
            is_open: false,
            modified_settings: settings.clone(),
            selected_tab: Default::default(),
            file_tab: Default::default(),
            analysis_tab: Default::default(),
            debug_tab: Default::default(),
            visuals_tab,
        }
    }

    pub fn show(
        &mut self,
        state: &mut AppState,
        selected_combat: Option<&Combat>,
        ui: &mut Ui,
        frame: &Frame,
    ) {
        if ui.selectable_label(self.is_open, "Settings").clicked() && !self.is_open {
            self.initialize(state);
        }

        self.handle_dropped_file(ui, state);
        if !self.is_open {
            return;
        }
        Window::new("Settings")
            .collapsible(false)
            .auto_sized()
            .max_size([1080.0, 720.0])
            .constrain(true)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.selected_tab, SettingsTab::File, "File");
                    ui.selectable_value(&mut self.selected_tab, SettingsTab::Analysis, "Analysis");
                    ui.selectable_value(&mut self.selected_tab, SettingsTab::Visuals, "Visuals");
                    ui.selectable_value(&mut self.selected_tab, SettingsTab::Debug, "Debug");
                });

                ui.separator();
                ScrollArea::both().show(ui, |ui| match self.selected_tab {
                    SettingsTab::File => self.file_tab.show(
                        &state.analysis_handler,
                        &mut self.modified_settings,
                        ui,
                        frame,
                    ),
                    SettingsTab::Analysis => {
                        self.analysis_tab
                            .show(&mut self.modified_settings, selected_combat, ui)
                    }
                    SettingsTab::Visuals => self.visuals_tab.show(&mut self.modified_settings, ui),
                    SettingsTab::Debug => self.debug_tab.show(&mut self.modified_settings, ui),
                });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Ok").clicked() {
                        self.apply_setting_changes(ui, state);
                    }

                    if ui.button("Cancel").clicked() {
                        self.discard_setting_changes(ui, state);
                    }
                })
            });
    }

    pub fn show_clear_log_dialog(&mut self, analysis_handler: &AnalysisHandler, ui: &mut Ui) {
        self.file_tab.show_clear_log_dialog(analysis_handler, ui);
    }

    fn handle_dropped_file(&mut self, ui: &mut Ui, state: &mut AppState) {
        ui.ctx().input(|i| {
            let file = i
                .raw
                .dropped_files
                .last()
                .map(|f| f.path.as_ref())
                .flatten();
            if let Some(file) = file {
                if file.extension() != Some(OsStr::new("log")) {
                    return;
                }
                if !self.is_open {
                    self.initialize(state);
                }
                self.modified_settings.analysis.combatlog_file = file.to_string_lossy().into();
                self.apply_setting_changes(ui, state);
            }
        });
    }

    fn initialize(&mut self, state: &AppState) {
        self.is_open = true;
        self.modified_settings = state.settings.clone();
        self.file_tab.initialize();
    }

    fn apply_setting_changes(&mut self, ui: &Ui, state: &mut AppState) {
        self.is_open = false;
        if self.modified_settings.analysis != state.settings.analysis
            || self.modified_settings.auto_refresh != state.settings.auto_refresh
        {
            state.analysis_handler = AnalysisHandler::new(
                self.modified_settings.analysis.clone(),
                ui.ctx().clone(),
                self.modified_settings.auto_refresh.interval_seconds(),
            );
            state.analysis_handler.refresh();
        }

        state.settings = self.modified_settings.clone();
        self.modified_settings.save();
    }

    fn discard_setting_changes(&mut self, ui: &Ui, state: &AppState) {
        self.is_open = false;
        if self.modified_settings.visuals != state.settings.visuals {
            self.visuals_tab.update_visuals(
                ui.ctx(),
                ui.ctx().native_pixels_per_point(),
                &state.settings,
            );
        }

        self.modified_settings = state.settings.clone();
    }
}
