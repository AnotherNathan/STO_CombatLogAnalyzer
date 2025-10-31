use eframe::egui::*;

use crate::{analyzer::*, custom_widgets::splitter::Splitter};

use super::{common::*, diagrams::*, tables::*};

pub struct DamageTab {
    table: DamageTable,
    dmg_main_diagrams: DamageDiagrams,
    dmg_selection_diagrams: Option<DamageDiagrams>,
    damage_group: for<'a> fn(&'a Player) -> &'a DamageGroup,
    filter: f64,
    diagram_time_slice: f64,
    active_diagram: DiagramType,
}

impl DamageTab {
    pub fn empty(damage_group: fn(&Player) -> &DamageGroup) -> Self {
        Self {
            table: DamageTable::empty(),
            dmg_main_diagrams: DamageDiagrams::empty(),
            damage_group: damage_group,
            filter: 0.4,
            diagram_time_slice: 1.0,
            dmg_selection_diagrams: None,
            active_diagram: DiagramType::Dps,
        }
    }

    pub fn update(&mut self, combat: &Combat) {
        self.table = DamageTable::new(combat, self.damage_group);
        self.dmg_main_diagrams = DamageDiagrams::from_damage_groups(
            combat.players.values().map(self.damage_group),
            combat,
            self.filter,
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
                        self.filter,
                        self.diagram_time_slice,
                    );
                });

                self.show_diagrams(bottom_ui);
            });
    }

    fn process_diagram_change(
        diagram: &mut Option<DamageDiagrams>,
        selection: TableSelectionEvent<DamageTablePartData>,
        filter: f64,
        damage_time_slice: f64,
    ) {
        match selection {
            TableSelectionEvent::Clear => *diagram = None,
            TableSelectionEvent::Group(part) => {
                *diagram = Some(Self::make_sub_parts_diagram_selection(
                    part,
                    filter,
                    damage_time_slice,
                ))
            }
            TableSelectionEvent::Single(part) => {
                *diagram = Some(Self::make_single_diagram_selection(
                    part,
                    filter,
                    damage_time_slice,
                ))
            }
            TableSelectionEvent::AddSingle(part) => match diagram.as_mut() {
                Some(diagram) => {
                    diagram.add_data(Self::make_single_data_set(part), filter, damage_time_slice);
                }
                None => {
                    *diagram = Some(Self::make_single_diagram_selection(
                        part,
                        filter,
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
        filter: f64,
        damage_time_slice: f64,
    ) -> DamageDiagrams {
        DamageDiagrams::from_data(
            part.sub_parts.iter().map(|p| {
                PreparedDamageDataSet::new(&p.name, part.total_damage(), p.source_hits.iter())
            }),
            filter,
            damage_time_slice,
        )
    }

    fn make_single_diagram_selection(
        part: &DamageTablePart,
        filter: f64,
        damage_time_slice: f64,
    ) -> DamageDiagrams {
        return DamageDiagrams::from_data(
            [Self::make_single_data_set(part)].into_iter(),
            filter,
            damage_time_slice,
        );
    }

    fn make_single_data_set(part: &DamageTablePart) -> PreparedDamageDataSet {
        PreparedDamageDataSet::new(&part.name, part.total_damage(), part.source_hits.iter())
    }

    fn update_diagrams(&mut self) {
        self.dmg_main_diagrams
            .update(self.filter, self.diagram_time_slice);
        if let Some(selection_plot) = &mut self.dmg_selection_diagrams {
            selection_plot.update(self.filter, self.diagram_time_slice);
        }
    }

    fn show_diagrams(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.active_diagram,
                DiagramType::Dps,
                DiagramType::Dps.name(),
            )
            .on_hover_text(DiagramType::Dps.tooltip());
            ui.selectable_value(
                &mut self.active_diagram,
                DiagramType::Damage,
                DiagramType::Damage.name(),
            )
            .on_hover_text(DiagramType::Damage.tooltip());
            ui.selectable_value(
                &mut self.active_diagram,
                DiagramType::DamageResistance,
                DiagramType::DamageResistance.name(),
            )
            .on_hover_text(DiagramType::DamageResistance.tooltip());
            ui.selectable_value(
                &mut self.active_diagram,
                DiagramType::HitsPerSecond,
                DiagramType::HitsPerSecond.name(),
            )
            .on_hover_text(DiagramType::HitsPerSecond.tooltip());
            ui.selectable_value(
                &mut self.active_diagram,
                DiagramType::HitsCount,
                DiagramType::HitsCount.name(),
            )
            .on_hover_text(DiagramType::HitsCount.tooltip());
        });

        let updated_required = match self.active_diagram {
            DiagramType::Damage | DiagramType::DamageResistance | DiagramType::HitsCount => {
                show_time_slice_setting(&mut self.diagram_time_slice, ui)
            }
            DiagramType::Dps | DiagramType::HitsPerSecond => {
                show_time_filter_setting(&mut self.filter, ui)
            }
            _ => unreachable!(),
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
