use eframe::egui::{ComboBox, Ui};

use crate::{
    analyzer::*,
    custom_widgets::{slider_text_edit::SliderTextEdit, splitter::Splitter},
};

use super::{damage_table::DamageTable, dps_plot::*};

pub struct DamageTab {
    table: DamageTable,
    dps_plot: DpsPlot,
    filter_size: f64,
    damage_group: fn(&Player) -> &DamageGroup,
    filter_method: FilterMethod,
}

impl DamageTab {
    pub fn empty(damage_group: fn(&Player) -> &DamageGroup) -> Self {
        Self {
            table: DamageTable::empty(),
            dps_plot: DpsPlot::empty(),
            filter_size: 2.0,
            damage_group: damage_group,
            filter_method: FilterMethod::Triangle,
        }
    }

    pub fn update(&mut self, combat: &Combat) {
        self.table = DamageTable::new(combat, self.damage_group);
        self.dps_plot = DpsPlot::from_groups(
            combat.players.values().map(self.damage_group),
            self.filter_size,
            self.filter_method,
        );
    }

    pub fn show(&mut self, ui: &mut Ui) {
        Splitter::horizontal()
            .initial_ratio(0.6)
            .ratio_bounds(0.1..=0.9)
            .show(ui, |top_ui, bottom_ui| {
                self.table.show(top_ui);

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
                    if SliderTextEdit::new(&mut self.filter_size, 2.0..=30.0, "filter slider")
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
                self.dps_plot.show(bottom_ui);
            });
    }

    fn update_plot(&mut self) {
        self.dps_plot.update(self.filter_size, self.filter_method);
    }
}
