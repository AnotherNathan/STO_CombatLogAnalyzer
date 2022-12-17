use std::{ops::Add, path::PathBuf, sync::Arc};

use chrono::Duration;
use eframe::egui::*;

use crate::analyzer::*;

use self::{
    damage_table::{DamageTable, TableColumns},
    settings::*,
};

mod damage_table;
mod settings;

#[derive(Default)]
pub struct App {
    settings: Settings,
    settings_window: SettingsWindow,
    analyzer: Option<Analyzer>,
    combats: Vec<Combat>,
    selected_combat: usize,
    table: Option<DamageTable>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let mut style = Style::clone(&cc.egui_ctx.style());
        style.override_font_id = Some(FontId::monospace(12.0));
        cc.egui_ctx.set_style(Arc::new(style));
        Default::default()
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            match self.settings_window.show(ctx, ui, &mut self.settings) {
                SettingsResult::NoChanges => (),
                SettingsResult::ReloadLog => {
                    self.analyzer = Analyzer::new(
                        &PathBuf::from(&self.settings.combatlog_file),
                        Duration::minutes(1).add(Duration::seconds(30)),
                    );
                    self.update_analysis();
                }
            }

            ui.horizontal(|ui| {
                ComboBox::new("combat list", "Combats")
                    .width(400.0)
                    .selected_text(
                        self.combats
                            .get(self.selected_combat)
                            .map(|c| c.identifier.as_str())
                            .unwrap_or(""),
                    )
                    .show_ui(ui, |ui| {
                        for (i, combat) in self.combats.iter().enumerate().rev() {
                            ui.selectable_value(&mut self.selected_combat, i, &combat.identifier);
                        }
                    });

                if ui.button("Refresh now").clicked() {
                    self.update_analysis();
                }
            });

            containers::scroll_area::ScrollArea::new([true, true]).show(ui, |ui| {
                let combat = match self.combats.get(self.selected_combat) {
                    Some(c) => c,
                    None => return,
                };

                let table = self.table.get_or_insert_with(|| DamageTable::new(combat));

                if table.identifier != combat.identifier {
                    *table = DamageTable::new(combat);
                }

                table.show(ui);
            })
        });
    }
}

impl App {
    fn update_analysis(&mut self) {
        // TODO run it in the background and then update

        if self.analyzer.is_none() {
            self.analyzer = Analyzer::new(
                &PathBuf::from(&self.settings.combatlog_file),
                Duration::minutes(1).add(Duration::seconds(40)),
            );
        }

        let analyzer = match self.analyzer.as_mut() {
            Some(a) => a,
            None => {
                return;
            }
        };

        analyzer.update();
        self.combats = analyzer.build_result();

        self.selected_combat = self.combats.len() - 1;
    }
}
