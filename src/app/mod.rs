use std::sync::Arc;

use eframe::egui::*;
use rfd::FileDialog;

use crate::analyzer::Combat;

use self::{
    analysis_handling::AnalysisInfo, main_tabs::*, overlay::Overlay, settings::*, state::AppState,
    status::*, summary_copy::SummaryCopy,
};

mod analysis_handling;
pub mod logging;
mod main_tabs;
mod overlay;
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
    overlay: Overlay,
    state: AppState,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let mut style = Style::clone(&cc.egui_ctx.style());
        style.override_font_id = Some(FontId::monospace(12.0));
        cc.egui_ctx.set_style(Arc::new(style));
        cc.egui_ctx
            .memory_mut(|m| m.options.repaint_on_widget_change = false);
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
            overlay: Default::default(),
            state,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        self.handle_analysis_infos(ctx);

        CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                self.settings_window.show(
                    &mut self.state,
                    self.selected_combat.as_ref(),
                    ui,
                    frame,
                );

                ui.horizontal_wrapped(|ui| {
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

                    self.settings_window
                        .show_clear_log_dialog(&self.state.analysis_handler, ui);

                    if ui
                        .checkbox(
                            &mut self.state.settings.auto_refresh.enable,
                            "Auto Refresh when log changes",
                        )
                        .clicked()
                    {
                        self.state
                            .analysis_handler
                            .set_auto_refresh(self.state.settings.auto_refresh.interval_seconds());
                        self.state.settings.save();
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
                            .set_file_name(
                                &self.selected_combat.as_ref().unwrap().file_identifier(),
                            )
                            .set_parent(frame)
                            .save_file()
                        {
                            self.state
                                .analysis_handler
                                .save_combat(self.selected_combat_index.unwrap(), file);
                        }
                    }

                    ui.separator();
                    self.summary_copy.show(self.selected_combat.as_ref(), ui);
                    ui.separator();
                    self.overlay.show(ui, self.selected_combat.as_ref());
                });

                self.main_tabs.show(ui);
            });
        });
    }

    fn clear_color(&self, _visuals: &eframe::egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}

impl App {
    fn handle_analysis_infos(&mut self, ctx: &Context) {
        let combatlog_file = &self.state.settings.analysis.combatlog_file;
        for info in self.state.analysis_handler.check_for_info() {
            match info {
                AnalysisInfo::Combat(combat) => {
                    self.main_tabs.update(&combat);
                    self.overlay.update(ctx, Some(&combat));
                    self.selected_combat = Some(combat);
                }
                AnalysisInfo::Refreshed {
                    latest_combat,
                    combats,
                    file_size,
                } => {
                    self.main_tabs.update(&latest_combat);
                    self.overlay.update(ctx, Some(&latest_combat));
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
