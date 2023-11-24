use std::sync::Arc;

use eframe::egui::*;
use rfd::FileDialog;

use crate::analyzer::Combat;

use self::{
    analysis_handling::AnalysisInfo, main_tabs::*, settings::*, state::AppState, status::*,
    summary_copy::SummaryCopy,
};

mod analysis_handling;
pub mod logging;
mod main_tabs;
mod settings;
mod state;
mod status;
mod summary_copy;

pub struct App {
    settings_window: SettingsWindow,
    combats: Vec<String>,
    selected_combat_index: Option<usize>,
    selected_combat: Option<Combat>,
    status_indicator: StatusIndicator,
    main_tabs: MainTabs,
    summary_copy: SummaryCopy,
    state: AppState,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let mut style = Style::clone(&cc.egui_ctx.style());
        style.override_font_id = Some(FontId::monospace(12.0));
        cc.egui_ctx.set_style(Arc::new(style));
        let state = AppState::new(&cc.egui_ctx);
        let settings_window =
            SettingsWindow::new(&cc.egui_ctx, cc.egui_ctx.native_pixels_per_point());
        Self {
            settings_window,
            combats: Default::default(),
            selected_combat_index: None,
            selected_combat: None,
            status_indicator: StatusIndicator::new(),
            main_tabs: MainTabs::empty(),
            summary_copy: Default::default(),
            state,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        self.handle_analysis_infos();

        CentralPanel::default().show(ctx, |ui| {
            self.settings_window
                .show(&mut self.state, self.selected_combat.as_ref(), ui, frame);

            ui.horizontal(|ui| {
                self.status_indicator
                    .show(self.state.analysis_handler.is_busy(), ui);

                ComboBox::new("combat list", "Combats")
                    .width(400.0)
                    .selected_text(self.main_tabs.identifier.as_str())
                    .show_ui(ui, |ui| {
                        for (i, combat) in self.combats.iter().enumerate().rev() {
                            if ui
                                .selectable_value(
                                    &mut self.selected_combat_index,
                                    Some(i),
                                    combat.as_str(),
                                )
                                .changed()
                            {
                                if let Some(combat_index) = self.selected_combat_index {
                                    self.state.analysis_handler.get_combat(combat_index);
                                }
                            }
                        }
                    });

                if ui.button("Refresh Now ⟲").clicked() {
                    self.state.analysis_handler.refresh();
                }

                if ui
                    .add_enabled(
                        self.selected_combat.is_some(),
                        Button::new("Save Combat 💾"),
                    )
                    .clicked()
                {
                    if let Some(file) = FileDialog::new()
                        .set_title("Save Combat")
                        .add_filter("log", &["log"])
                        .set_file_name(&self.selected_combat.as_ref().unwrap().file_identifier())
                        .set_parent(frame)
                        .save_file()
                    {
                        self.state
                            .analysis_handler
                            .save_combat(self.selected_combat_index.unwrap(), file);
                    }
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    self.summary_copy.show(self.selected_combat.as_ref(), ui);
                });
            });

            self.main_tabs.show(ui);
        });
    }
}

impl App {
    fn handle_analysis_infos(&mut self) {
        let combatlog_file = &self.state.settings.analysis.combatlog_file;
        for info in self.state.analysis_handler.check_for_info() {
            match info {
                AnalysisInfo::Combat(combat) => {
                    self.main_tabs.update(&combat);
                    self.selected_combat = Some(combat);
                }
                AnalysisInfo::Refreshed {
                    latest_combat,
                    combats,
                    file_size,
                } => {
                    self.main_tabs.update(&latest_combat);
                    self.combats = combats;
                    self.selected_combat_index = Some(self.combats.len() - 1);
                    self.selected_combat = Some(latest_combat);
                    self.status_indicator.status = Status::Loaded {
                        combatlog_file: combatlog_file.clone(),
                        file_size,
                    };
                }
                AnalysisInfo::RefreshError => {
                    self.status_indicator.status = Status::LoadError {
                        combatlog_file: combatlog_file.clone(),
                    };
                }
            }
        }
    }
}
