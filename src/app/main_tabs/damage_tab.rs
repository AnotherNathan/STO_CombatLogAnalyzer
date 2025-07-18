use eframe::egui::*;

use crate::{analyzer::*, custom_widgets::splitter::Splitter};

use super::{common::*, diagrams::*, tables::*};

pub struct DamageTab {
    table: DamageTable,
    dmg_main_diagrams: DamageDiagrams,
    dmg_selection_diagrams: Option<DamageDiagrams>,
    damage_group: for<'a> fn(&'a Player) -> &'a DamageGroup,
    dps_filter: f64,
    diagram_time_slice: f64,
    active_diagram: ActiveDamageDiagram,
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
        }
    }

    pub fn update(&mut self, combat: &Combat) {
        self.table = DamageTable::new(combat, self.damage_group);
        self.dmg_main_diagrams = DamageDiagrams::from_damage_groups(
            combat.players.values().map(self.damage_group),
            combat,
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
                    Self::process_diagram_change(
                        &mut self.dmg_selection_diagrams,
                        p,
                        self.dps_filter,
                        self.diagram_time_slice,
                    );
                });

                self.show_diagrams(bottom_ui);
            });
    }

    fn process_diagram_change(
        diagram: &mut Option<DamageDiagrams>,
        selection: TableSelectionEvent<DamageTablePartData>,
        dps_filter: f64,
        damage_time_slice: f64,
    ) {
        match selection {
            TableSelectionEvent::Clear => *diagram = None,
            TableSelectionEvent::Group(part) => {
                *diagram = Some(Self::make_sub_parts_diagram_selection(
                    part,
                    dps_filter,
                    damage_time_slice,
                ))
            }
            TableSelectionEvent::Single(part) => {
                *diagram = Some(Self::make_single_diagram_selection(
                    part,
                    dps_filter,
                    damage_time_slice,
                ))
            }
            TableSelectionEvent::AddSingle(part) => match diagram.as_mut() {
                Some(diagram) => {
                    diagram.add_data(
                        Self::make_single_data_set(part),
                        dps_filter,
                        damage_time_slice,
                    );
                }
                None => {
                    *diagram = Some(Self::make_single_diagram_selection(
                        part,
                        dps_filter,
                        damage_time_slice,
                    ))
                }
            },
            TableSelectionEvent::Unselect(part) => {
                if let Some(diagram) = diagram.as_mut() {
                    diagram.remove_data(part);
                }
            }
        }
    }

    fn make_sub_parts_diagram_selection(
        part: &DamageTablePart,
        dps_filter: f64,
        damage_time_slice: f64,
    ) -> DamageDiagrams {
        DamageDiagrams::from_data(
            part.sub_parts.iter().map(|p| {
                PreparedDamageDataSet::new(&p.name, part.total_damage(), p.source_hits.iter())
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
            [Self::make_single_data_set(part)].into_iter(),
            dps_filter,
            damage_time_slice,
        );
    }

    fn make_single_data_set(part: &DamageTablePart) -> PreparedDamageDataSet {
        PreparedDamageDataSet::new(&part.name, part.total_damage(), part.source_hits.iter())
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

        let updated_required = match self.active_diagram {
            ActiveDamageDiagram::Damage | ActiveDamageDiagram::DamageResistance => {
                show_time_slice_setting(&mut self.diagram_time_slice, ui)
            }
            ActiveDamageDiagram::Dps => show_time_filter_setting(&mut self.dps_filter, ui),
        };

        if updated_required {
            self.update_diagrams();
        }

        if let Some(selection_diagrams) = &mut self.dmg_selection_diagrams {
            selection_diagrams.show(ui, self.active_diagram);
        } else {
            self.dmg_main_diagrams.show(ui, self.active_diagram);
        }
    }
}
