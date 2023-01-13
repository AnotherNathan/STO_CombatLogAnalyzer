use eframe::egui::*;

use crate::{
    analyzer::*,
    custom_widgets::{slider_text_edit::SliderTextEdit, splitter::Splitter},
};

use super::{
    damage_table::{DamageTable, TablePart},
    dps_plot::*,
};

pub struct DamageTab {
    table: DamageTable,
    dps_main_plot: DpsPlot,
    dps_selection_plot: Option<DpsPlot>,
    damage_group: fn(&Player) -> &DamageGroup,
    filter: f64,
}

impl DamageTab {
    pub fn empty(damage_group: fn(&Player) -> &DamageGroup) -> Self {
        Self {
            table: DamageTable::empty(),
            dps_main_plot: DpsPlot::empty(),
            damage_group: damage_group,
            filter: 0.4,
            dps_selection_plot: None,
        }
    }

    pub fn update(&mut self, combat: &Combat) {
        self.table = DamageTable::new(combat, self.damage_group);
        self.dps_main_plot = DpsPlot::from_damage_groups(
            combat.players.values().map(self.damage_group),
            self.filter,
        );
        self.dps_selection_plot = None;
    }

    pub fn show(&mut self, ui: &mut Ui) {
        Splitter::horizontal()
            .initial_ratio(0.6)
            .ratio_bounds(0.1..=0.9)
            .show(ui, |top_ui, bottom_ui| {
                self.table.show(top_ui, |p| {
                    self.dps_selection_plot = Self::make_selection_plot(p, self.filter);
                });

                bottom_ui.horizontal(|ui| {
                    if SliderTextEdit::new(&mut self.filter, 0.4..=6.0, "filter slider")
                        .clamp_min(0.1)
                        .clamp_max(120.0)
                        .desired_text_edit_width(30.0)
                        .display_precision(4)
                        .step_by(0.1)
                        .show(ui)
                        .changed()
                    {
                        self.update_plot();
                    }
                    ui.label("Gauss Filter Standard Deviation (how much to smooth the DPS graph)");
                });

                if let Some(selected_plot) = &mut self.dps_selection_plot {
                    selected_plot.show(bottom_ui);
                } else {
                    self.dps_main_plot.show(bottom_ui);
                }
            });
    }

    fn make_selection_plot(part: Option<&TablePart>, filter: f64) -> Option<DpsPlot> {
        let part = part?;
        if part.sub_parts.len() == 0 {
            return Some(DpsPlot::from_data(
                [(part.name.as_str(), part.source_hits.iter())].into_iter(),
                filter,
            ));
        }

        Some(DpsPlot::from_data(
            part.sub_parts
                .iter()
                .map(|p| (p.name.as_str(), p.source_hits.iter())),
            filter,
        ))
    }

    fn update_plot(&mut self) {
        self.dps_main_plot.update(self.filter);
        if let Some(selection_plot) = &mut self.dps_selection_plot {
            selection_plot.update(self.filter);
        }
    }
}
