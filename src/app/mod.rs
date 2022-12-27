use std::{ops::Add, path::PathBuf, sync::Arc};

use chrono::Duration;
use eframe::egui::*;

use crate::analyzer::*;

use self::{
    analysis_handling::{AnalysisHandler, AnalysisInfo},
    damage_table::{DamageTable, TableColumns},
    settings::*,
};

mod analysis_handling;
mod damage_table;
pub mod logging;
mod settings;

pub struct App {
    settings: Settings,
    settings_window: SettingsWindow,
    combats: Vec<String>,
    selected_combat: Option<usize>,
    table: DamageTable,
    analysis_handler: AnalysisHandler,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let mut style = Style::clone(&cc.egui_ctx.style());
        style.override_font_id = Some(FontId::monospace(12.0));
        cc.egui_ctx.set_style(Arc::new(style));
        let settings = Settings::load_or_default();
        let analysis_handler = AnalysisHandler::new(settings.analysis.clone(), cc.egui_ctx.clone());
        Self {
            settings,
            settings_window: Default::default(),
            combats: Default::default(),
            selected_combat: None,
            table: DamageTable::empty(),
            analysis_handler,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        self.handle_analysis_infos();

        CentralPanel::default().show(ctx, |ui| {
            match self.settings_window.show(ctx, ui, &mut self.settings) {
                SettingsResult::NoChanges => (),
                SettingsResult::ReloadLog => {
                    self.analysis_handler
                        .update_settings_and_refresh(self.settings.analysis.clone());
                }
            }

            ui.horizontal(|ui| {
                ComboBox::new("combat list", "Combats")
                    .width(400.0)
                    .selected_text(self.table.identifier.as_str())
                    .show_ui(ui, |ui| {
                        for (i, combat) in self.combats.iter().enumerate().rev() {
                            if ui
                                .selectable_value(
                                    &mut self.selected_combat,
                                    Some(i),
                                    combat.as_str(),
                                )
                                .changed()
                            {
                                if let Some(combat_index) = self.selected_combat {
                                    self.analysis_handler.get_combat(combat_index);
                                }
                            }
                        }
                    });

                if ui.button("Refresh now").clicked() {
                    self.analysis_handler.refresh();
                }

                if self.analysis_handler.is_busy() {
                    ui.label("⏳ Refreshing..");
                }
            });

            containers::scroll_area::ScrollArea::new([true, true]).show(ui, |ui| {
                self.table.show(ui);
            })
        });
    }
}

impl App {
    fn handle_analysis_infos(&mut self) {
        for info in self.analysis_handler.check_for_info() {
            match info {
                AnalysisInfo::Combat(combat) => self.table = DamageTable::new(&combat),
                AnalysisInfo::Refreshed {
                    latest_combat,
                    combats,
                } => {
                    self.table = DamageTable::new(&latest_combat);
                    self.combats = combats;
                    self.selected_combat = Some(self.combats.len());
                }
            }
        }
    }
}
