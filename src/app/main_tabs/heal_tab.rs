use eframe::egui::Ui;

use crate::{analyzer::*, custom_widgets::splitter::Splitter};

use super::{common::*, diagrams::*, tables::*};

pub struct HealTab {
    table: HealTable,
    main_diagrams: HealDiagrams,
    selection_diagrams: Option<HealDiagrams>,
    heal_group: fn(&Player) -> &HealGroup,
    hps_filter: f64,
    diagram_time_slice: f64,
    active_diagram: ActiveHealDiagram,
}

impl HealTab {
    pub fn empty(heal_group: fn(&Player) -> &HealGroup) -> Self {
        Self {
            table: HealTable::empty(),
            heal_group,
            main_diagrams: HealDiagrams::empty(),
            selection_diagrams: None,
            hps_filter: 0.4,
            diagram_time_slice: 1.0,
            active_diagram: ActiveHealDiagram::Heal,
        }
    }

    pub fn update(&mut self, combat: &Combat) {
        self.table = HealTable::new(combat, self.heal_group);
        self.main_diagrams = HealDiagrams::from_heal_groups(
            combat.players.values().map(self.heal_group),
            combat,
            self.hps_filter,
            self.diagram_time_slice,
        );
        self.selection_diagrams = None;
    }

    pub fn show(&mut self, ui: &mut Ui) {
        Splitter::horizontal()
            .initial_ratio(0.6)
            .ratio_bounds(0.1..=0.9)
            .show(ui, |top_ui, bottom_ui| {
                self.table.show(top_ui, |p| {
                    Self::process_diagram_change(
                        &mut self.selection_diagrams,
                        p,
                        self.hps_filter,
                        self.diagram_time_slice,
                    );
                });

                self.show_diagrams(bottom_ui);
            });
    }

    fn process_diagram_change(
        diagram: &mut Option<HealDiagrams>,
        selection: TableSelectionEvent<HealTablePartData>,
        hps_filter: f64,
        heal_time_slice: f64,
    ) {
        match selection {
            TableSelectionEvent::Clear => *diagram = None,
            TableSelectionEvent::Group(part) => {
                *diagram = Some(Self::make_sub_parts_diagram_selection(
                    part,
                    hps_filter,
                    heal_time_slice,
                ))
            }
            TableSelectionEvent::Single(part) => {
                *diagram = Some(Self::make_single_diagram_selection(
                    part,
                    hps_filter,
                    heal_time_slice,
                ))
            }
            TableSelectionEvent::AddSingle(part) => match diagram.as_mut() {
                Some(diagram) => {
                    diagram.add_data(
                        Self::make_single_data_set(part),
                        hps_filter,
                        heal_time_slice,
                    );
                }
                None => {
                    *diagram = Some(Self::make_single_diagram_selection(
                        part,
                        hps_filter,
                        heal_time_slice,
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
        part: &HealTablePart,
        hps_filter: f64,
        heal_time_slice: f64,
    ) -> HealDiagrams {
        HealDiagrams::from_data(
            part.sub_parts.iter().map(|p| {
                PreparedHealDataSet::new(&p.name, part.total_heal(), p.source_ticks.iter())
            }),
            hps_filter,
            heal_time_slice,
        )
    }

    fn make_single_diagram_selection(
        part: &HealTablePart,
        hps_filter: f64,
        heal_time_slice: f64,
    ) -> HealDiagrams {
        return HealDiagrams::from_data(
            [Self::make_single_data_set(part)].into_iter(),
            hps_filter,
            heal_time_slice,
        );
    }

    fn make_single_data_set(part: &HealTablePart) -> PreparedHealDataSet {
        PreparedHealDataSet::new(&part.name, part.total_heal(), part.source_ticks.iter())
    }

    fn update_diagrams(&mut self) {
        self.main_diagrams
            .update(self.hps_filter, self.diagram_time_slice);
        if let Some(selection_plot) = &mut self.selection_diagrams {
            selection_plot.update(self.hps_filter, self.diagram_time_slice);
        }
    }

    fn show_diagrams(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.active_diagram,
                ActiveHealDiagram::Heal,
                ActiveHealDiagram::Heal.display(),
            );
            ui.selectable_value(
                &mut self.active_diagram,
                ActiveHealDiagram::Hps,
                ActiveHealDiagram::Hps.display(),
            );
        });

        let update_required = match self.active_diagram {
            ActiveHealDiagram::Heal => show_time_slice_setting(&mut self.diagram_time_slice, ui),
            ActiveHealDiagram::Hps => show_time_filter_setting(&mut self.hps_filter, ui),
        };

        if update_required {
            self.update_diagrams();
        }

        if let Some(selection_diagrams) = &mut self.selection_diagrams {
            selection_diagrams.show(ui, self.active_diagram);
        } else {
            self.main_diagrams.show(ui, self.active_diagram);
        }
    }
}
