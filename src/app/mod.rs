use std::sync::Arc;

use eframe::egui::*;

use crate::analyzer::*;

use self::{
    analysis_handling::AnalysisInfo, damage_table::DamageTable, settings::*, state::AppState,
};

mod analysis_handling;
mod damage_table;
pub mod logging;
mod settings;
mod state;

pub struct App {
    settings_window: SettingsWindow,
    combats: Vec<String>,
    selected_combat: Option<usize>,
    tables: DamageTables,
    active_tab: Tab,
    state: AppState,
}

struct DamageTables {
    identifier: String,
    damage_out: DamageTable,
    damage_in: DamageTable,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Tab {
    #[default]
    DamageOut,
    DamageIn,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let mut style = Style::clone(&cc.egui_ctx.style());
        style.override_font_id = Some(FontId::monospace(12.0));
        cc.egui_ctx.set_style(Arc::new(style));
        let state = AppState::new(&cc.egui_ctx);
        let settings_window =
            SettingsWindow::new(&cc.egui_ctx, cc.integration_info.native_pixels_per_point);
        Self {
            settings_window,
            combats: Default::default(),
            selected_combat: None,
            tables: DamageTables::empty(),
            active_tab: Default::default(),
            state,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        self.handle_analysis_infos();

        CentralPanel::default().show(ctx, |ui| {
            self.settings_window.show(&mut self.state, ui, frame);

            ui.horizontal(|ui| {
                ComboBox::new("combat list", "Combats")
                    .width(400.0)
                    .selected_text(self.tables.identifier.as_str())
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
                                    self.state.analysis_handler.get_combat(combat_index);
                                }
                            }
                        }
                    });

                if ui.button("Refresh now").clicked() {
                    self.state.analysis_handler.refresh();
                }

                if self.state.analysis_handler.is_busy() {
                    ui.label("⏳ Working..");
                }
            });

            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, Tab::DamageOut, "Outgoing Damage");

                ui.selectable_value(&mut self.active_tab, Tab::DamageIn, "Incoming Damage");
            });

            match self.active_tab {
                Tab::DamageIn => self.tables.damage_in.show(ui),
                Tab::DamageOut => self.tables.damage_out.show(ui),
            }
        });
    }
}

impl App {
    fn handle_analysis_infos(&mut self) {
        for info in self.state.analysis_handler.check_for_info() {
            match info {
                AnalysisInfo::Combat(combat) => self.tables.update(&combat),
                AnalysisInfo::Refreshed {
                    latest_combat,
                    combats,
                } => {
                    self.tables.update(&latest_combat);
                    self.combats = combats;
                    self.selected_combat = Some(self.combats.len());
                }
            }
        }
    }
}

impl DamageTables {
    fn empty() -> Self {
        Self {
            identifier: "<no data loaded>".to_string(),
            damage_out: DamageTable::empty(),
            damage_in: DamageTable::empty(),
        }
    }

    fn update(&mut self, combat: &Combat) {
        self.identifier = combat.identifier();
        self.damage_out = DamageTable::new(combat, |p| &p.damage_out);
        self.damage_in = DamageTable::new(combat, |p| &p.damage_in);
    }
}
