use eframe::egui::*;

use crate::{
    analyzer::*,
    custom_widgets::{slider_text_edit::SliderTextEdit, splitter::Splitter},
};

use super::{diagrams::*, tables::*};

pub struct DamageTab {
    table: DamageTable,
    dmg_main_diagrams: DamageDiagrams,
    dmg_selection_diagrams: Option<DamageDiagrams>,
    damage_group: fn(&Player) -> &DamageGroup,
    dps_filter: f64,
    diagram_time_slice: f64,
    active_diagram: ActiveDamageDiagram,
    damage_resistance_selection: DamageResistanceSelection,
}

impl DamageTab {
    pub fn empty(damage_group: fn(&Player) -> &DamageGroup) -> Self {
        Self {
            table: DamageTable::empty(),
            dmg_main_diagrams: DamageDiagrams::empty(),
            damage_group: damage_group,
            dps_filter: 0.4,
            diagram_time_slice: 1.0,
            dmg_selection_diagrams: None,
            active_diagram: ActiveDamageDiagram::Damage,
            damage_resistance_selection: Default::default(),
        }
    }

    pub fn update(&mut self, combat: &Combat) {
        self.table = DamageTable::new(combat, self.damage_group);
        self.dmg_main_diagrams = DamageDiagrams::from_damage_groups(
            combat.players.values().map(self.damage_group),
            self.dps_filter,
            self.diagram_time_slice,
        );
        self.dmg_selection_diagrams = None;
    }

    pub fn show(&mut self, ui: &mut Ui) {
        Splitter::horizontal()
            .initial_ratio(0.6)
            .ratio_bounds(0.1..=0.9)
            .show(ui, |top_ui, bottom_ui| {
                self.table.show(top_ui, |p| {
                    self.dmg_selection_diagrams =
                        Self::make_selection_diagrams(p, self.dps_filter, self.diagram_time_slice);
                });

                self.show_diagrams(bottom_ui);
            });
    }

    fn make_selection_diagrams(
        selection: TableSelection,
        dps_filter: f64,
        damage_time_slice: f64,
    ) -> Option<DamageDiagrams> {
        match selection {
            TableSelection::SubPartsOrSingle(part) if part.sub_parts.len() == 0 => Some(
                Self::make_single_diagram_selection(part, dps_filter, damage_time_slice),
            ),
            TableSelection::Single(part) => Some(Self::make_single_diagram_selection(
                part,
                dps_filter,
                damage_time_slice,
            )),
            TableSelection::SubPartsOrSingle(part) => Some(Self::make_sub_parts_diagram_selection(
                part,
                dps_filter,
                damage_time_slice,
            )),
            TableSelection::Unselect => None,
        }
    }

    fn make_sub_parts_diagram_selection(
        part: &DamageTablePart,
        dps_filter: f64,
        damage_time_slice: f64,
    ) -> DamageDiagrams {
        DamageDiagrams::from_data(
            part.sub_parts.iter().map(|p| {
                PreparedDamageDataSet::new(
                    &p.name,
                    part.dps(),
                    part.total_damage(),
                    p.source_hits.iter(),
                )
            }),
            dps_filter,
            damage_time_slice,
        )
    }

    fn make_single_diagram_selection(
        part: &DamageTablePart,
        dps_filter: f64,
        damage_time_slice: f64,
    ) -> DamageDiagrams {
        return DamageDiagrams::from_data(
            [PreparedDamageDataSet::new(
                &part.name,
                part.dps(),
                part.total_damage(),
                part.source_hits.iter(),
            )]
            .into_iter(),
            dps_filter,
            damage_time_slice,
        );
    }

    fn update_diagrams(&mut self) {
        self.dmg_main_diagrams
            .update(self.dps_filter, self.diagram_time_slice);
        if let Some(selection_plot) = &mut self.dmg_selection_diagrams {
            selection_plot.update(self.dps_filter, self.diagram_time_slice);
        }
    }

    fn show_diagrams(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.active_diagram,
                ActiveDamageDiagram::Damage,
                ActiveDamageDiagram::Damage.display(),
            );
            ui.selectable_value(
                &mut self.active_diagram,
                ActiveDamageDiagram::Dps,
                ActiveDamageDiagram::Dps.display(),
            );
            ui.selectable_value(
                &mut self.active_diagram,
                ActiveDamageDiagram::DamageResistance,
                ActiveDamageDiagram::DamageResistance.display(),
            );
        });

        match self.active_diagram {
            ActiveDamageDiagram::Damage => {
                self.show_time_slice_setting(ui);
            }
            ActiveDamageDiagram::Dps => {
                self.show_time_filter_setting(ui);
            }
            ActiveDamageDiagram::DamageResistance => {
                self.show_time_slice_setting(ui);
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.damage_resistance_selection,
                        DamageResistanceSelection::All,
                        DamageResistanceSelection::All.display(),
                    );
                    ui.selectable_value(
                        &mut self.damage_resistance_selection,
                        DamageResistanceSelection::Hull,
                        DamageResistanceSelection::Hull.display(),
                    );
                    ui.selectable_value(
                        &mut self.damage_resistance_selection,
                        DamageResistanceSelection::Shield,
                        DamageResistanceSelection::Shield.display(),
                    );
                });
            }
        }

        if let Some(selection_diagrams) = &mut self.dmg_selection_diagrams {
            selection_diagrams.show(ui, self.active_diagram, self.damage_resistance_selection);
        } else {
            self.dmg_main_diagrams
                .show(ui, self.active_diagram, self.damage_resistance_selection);
        }
    }

    fn show_time_slice_setting(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if SliderTextEdit::new(&mut self.diagram_time_slice, 0.1..=6.0, "time slice slider")
                .clamp_min(0.1)
                .clamp_max(120.0)
                .desired_text_edit_width(30.0)
                .display_precision(4)
                .step_by(0.1)
                .show(ui)
                .changed()
            {
                self.update_diagrams();
            }
            ui.label("Time Slice (s)");
        });
    }

    fn show_time_filter_setting(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if SliderTextEdit::new(&mut self.dps_filter, 0.4..=6.0, "filter slider")
                .clamp_min(0.1)
                .clamp_max(120.0)
                .desired_text_edit_width(30.0)
                .display_precision(4)
                .step_by(0.1)
                .show(ui)
                .changed()
            {
                self.update_diagrams();
            }
            ui.label("Gauss Filter Standard Deviation (how much to smooth the DPS graph)");
        });
    }
}
