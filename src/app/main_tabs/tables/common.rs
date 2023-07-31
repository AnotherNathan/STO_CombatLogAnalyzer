use eframe::egui::*;

use crate::{analyzer::*, app::main_tabs::common::ROW_HEIGHT, custom_widgets::table::*};

pub struct Kills {
    total: String,
    pub total_count: u32,
    kills: Vec<(String, String)>,
}

impl Kills {
    pub fn new(source: &DamageGroup, name_manager: &NameManager) -> Self {
        let total_kills: u32 = source.kills.values().copied().sum();

        let kills = source
            .kills
            .iter()
            .map(|(n, k)| (name_manager.name(*n).to_string(), k.to_string()))
            .collect();
        Self {
            total: total_kills.to_string(),
            total_count: total_kills,
            kills,
        }
    }

    pub fn show(&self, row: &mut TableRow) {
        let response = row.cell_with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(&self.total);
        });

        if self.total_count > 0 {
            response.on_hover_ui(|ui| {
                Table::new(ui).body(ROW_HEIGHT, |b| {
                    for (name, count) in self.kills.iter() {
                        b.row(|r| {
                            r.cell(|ui| {
                                ui.label(name.as_str());
                            });
                            r.cell(|ui| {
                                ui.label(count.as_str());
                            });
                        });
                    }
                });
            });
        }
    }
}
