use eframe::egui::{ComboBox, Ui};

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
    filter_size_s: f64,
    damage_group: fn(&Player) -> &DamageGroup,
    filter_method: FilterMethod,
}

impl DamageTab {
    pub fn empty(damage_group: fn(&Player) -> &DamageGroup) -> Self {
        Self {
            table: DamageTable::empty(),
            dps_main_plot: DpsPlot::empty(),
            filter_size_s: 2.0,
            damage_group: damage_group,
            filter_method: FilterMethod::Triangle,
            dps_selection_plot: None,
        }
    }

    pub fn update(&mut self, combat: &Combat) {
        self.table = DamageTable::new(combat, self.damage_group);
        self.dps_main_plot = DpsPlot::from_damage_groups(
            combat.players.values().map(self.damage_group),
            self.filter_size_s,
            self.filter_method,
        );
        self.dps_selection_plot = None;
    }

    pub fn show(&mut self, ui: &mut Ui) {
        Splitter::horizontal()
            .initial_ratio(0.6)
            .ratio_bounds(0.1..=0.9)
            .show(ui, |top_ui, bottom_ui| {
                self.table.show(top_ui, |p| {
                    self.dps_selection_plot =
                        Self::make_selection_plot(p, self.filter_size_s, self.filter_method);
                });

                bottom_ui.horizontal(|ui| {
                    if ComboBox::from_label("Filter Method")
                        .selected_text(self.filter_method.display())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.filter_method,
                                FilterMethod::Triangle,
                                FilterMethod::Triangle.display(),
                            ) | ui.selectable_value(
                                &mut self.filter_method,
                                FilterMethod::Box,
                                FilterMethod::Box.display(),
                            )
                        })
                        .inner
                        .map(|r| r.changed())
                        .unwrap_or(false)
                    {
                        self.update_plot();
                    }
                });
                bottom_ui.horizontal(|ui| {
                    if SliderTextEdit::new(&mut self.filter_size_s, 2.0..=30.0, "filter slider")
                        .clamp_min(0.1)
                        .desired_text_edit_width(30.0)
                        .display_precision(4)
                        .step_by(0.1)
                        .show(ui)
                        .changed()
                    {
                        self.update_plot();
                    }
                    ui.label("Filter Size (s)");
                });

                if let Some(selected_plot) = &mut self.dps_selection_plot {
                    selected_plot.show(bottom_ui);
                } else {
                    self.dps_main_plot.show(bottom_ui);
                }
            });
    }

    fn make_selection_plot(
        part: Option<&TablePart>,
        filter_size_s: f64,
        filter_method: FilterMethod,
    ) -> Option<DpsPlot> {
        let part = part?;
        if part.sub_parts.len() == 0 {
            return Some(DpsPlot::from_data(
                [(part.name.as_str(), part.source_hits.iter())].into_iter(),
                filter_size_s,
                filter_method,
            ));
        }

        Some(DpsPlot::from_data(
            part.sub_parts
                .iter()
                .map(|p| (p.name.as_str(), p.source_hits.iter())),
            filter_size_s,
            filter_method,
        ))
    }

    fn update_plot(&mut self) {
        self.dps_main_plot
            .update(self.filter_size_s, self.filter_method);
        if let Some(selection_plot) = &mut self.dps_selection_plot {
            selection_plot.update(self.filter_size_s, self.filter_method);
        }
    }
}
