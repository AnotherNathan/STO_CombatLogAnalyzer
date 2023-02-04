use std::cmp::Reverse;

use eframe::egui::*;
use egui_extras::{Column, TableBody, TableBuilder, TableRow};

use crate::{
    analyzer::*,
    app::main_tabs::common::*,
    helpers::{number_formatting::NumberFormatter, F64TotalOrd},
};

macro_rules! col {
    ($name:expr, $sort:expr, $show:expr $(,)?) => {
        ColumnDescriptor {
            name: $name,
            sort: $sort,
            show: $show,
        }
    };
}

static COLUMNS: &[ColumnDescriptor] = &[
    col!(
        "Total Damage",
        |t| t.sort_by_option_f64_desc(|p| p.total_damage.all.value),
        |t, r| t.total_damage.show(r),
    ),
    col!(
        "DPS",
        |t| t.sort_by_option_f64_desc(|p| p.dps.all.value),
        |t, r| t.dps.show(r),
    ),
    col!(
        "Damage %",
        |t| t.sort_by_option_f64_desc(|p| p.damage_percentage.value),
        |t, r| {
            t.damage_percentage.show(r);
        },
    ),
    col!(
        "Damage Resistance %",
        |t| t.sort_by_option_f64_asc(|p| p.damage_resistance_percentage.all.value),
        |t, r| t.damage_resistance_percentage.show(r),
    ),
    col!(
        "Damage Resistance",
        |t| t.sort_by_option_f64_asc(|p| p.damage_resistance.all.value),
        |t, r| t.damage_resistance.show(r),
    ),
    col!(
        "Max One-Hit",
        |t| t.sort_by_option_f64_desc(|p| p.max_one_hit.damage.value),
        |t, r| t.max_one_hit.show(r),
    ),
    col!(
        "Average Hit",
        |t| t.sort_by_option_f64_desc(|p| p.average_hit.all.value),
        |t, r| t.average_hit.show(r),
    ),
    col!(
        "Critical Chance %",
        |t| t.sort_by_option_f64_desc(|p| p.critical_chance.value),
        |t, r| {
            t.critical_chance.show(r);
        },
    ),
    col!(
        "Flanking %",
        |t| t.sort_by_option_f64_desc(|p| p.flanking.value),
        |t, r| {
            t.flanking.show(r);
        },
    ),
    col!("Hits", |t| t.sort_by_desc(|p| p.hits.all), |t, r| {
        t.hits.show(r);
    },),
];

pub struct DamageTable {
    players: Vec<DamageTablePart>,
    selected_id: Option<u32>,
}

pub struct DamageTablePart {
    pub name: String,
    total_damage: ShieldAndHullTextValue,
    dps: ShieldAndHullTextValue,
    damage_percentage: TextValue,
    max_one_hit: MaxOneHit,
    average_hit: ShieldAndHullTextValue,
    critical_chance: TextValue,
    flanking: TextValue,
    damage_resistance_percentage: ShieldAndHullTextValue,
    damage_resistance: ShieldAndHullTextValue,
    hits: Hits,
    pub sub_parts: Vec<DamageTablePart>,

    pub source_hits: Vec<Hit>,

    id: u32,

    open: bool,
}

pub enum TableSelection<'a> {
    SubPartsOrSingle(&'a DamageTablePart),
    Single(&'a DamageTablePart),
    Unselect,
}

struct MaxOneHit {
    damage: TextValue,
    name: String,
}

struct Hits {
    all: u64,
    all_text: String,
    shield: String,
    hull: String,
}

struct ColumnDescriptor {
    name: &'static str,
    sort: fn(&mut DamageTable),
    show: fn(&mut DamageTablePart, &mut TableRow),
}

impl DamageTable {
    pub fn empty() -> Self {
        Self {
            players: Vec::new(),
            selected_id: None,
        }
    }

    pub fn new(combat: &Combat, mut damage_group: impl FnMut(&Player) -> &DamageGroup) -> Self {
        let mut number_formatter = NumberFormatter::new();
        let mut id_source = 0;
        let mut table = Self {
            players: combat
                .players
                .values()
                .map(|p| {
                    DamageTablePart::new(damage_group(p), &mut number_formatter, &mut id_source)
                })
                .collect(),
            selected_id: None,
        };
        (COLUMNS[0].sort)(&mut table);

        table
    }

    pub fn show(&mut self, ui: &mut Ui, mut on_selected: impl FnMut(TableSelection)) {
        ScrollArea::horizontal()
            .min_scrolled_width(0.0)
            .show(ui, |ui| {
                TableBuilder::new(ui)
                    .columns(Column::auto(), COLUMNS.len() + 1)
                    .striped(true)
                    .min_scrolled_height(0.0)
                    .max_scroll_height(f32::MAX)
                    .header(0.0, |mut r| {
                        r.col(|ui| {
                            ui.label("Name");
                        });

                        for column in COLUMNS.iter() {
                            self.show_column_header(&mut r, column);
                        }
                    })
                    .body(|mut t| {
                        for player in self.players.iter_mut() {
                            player.show(&mut t, 0.0, &mut self.selected_id, &mut on_selected);
                        }
                    });
            });
    }

    fn show_column_header(&mut self, row: &mut TableRow, column: &ColumnDescriptor) {
        row.col(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.selectable_label(false, column.name).clicked() {
                    (column.sort)(self);
                }
            });
        });
    }

    fn sort_by_option_f64_desc(
        &mut self,
        mut key: impl FnMut(&DamageTablePart) -> Option<f64> + Copy,
    ) {
        self.sort_by_desc(move |p| key(p).map(|v| F64TotalOrd(v)));
    }

    fn sort_by_option_f64_asc(
        &mut self,
        mut key: impl FnMut(&DamageTablePart) -> Option<f64> + Copy,
    ) {
        self.sort_by_asc(move |p| key(p).map(|v| F64TotalOrd(v)));
    }

    fn sort_by_desc<K: Ord>(&mut self, mut key: impl FnMut(&DamageTablePart) -> K + Copy) {
        self.players.sort_unstable_by_key(|p| Reverse(key(p)));

        self.players.iter_mut().for_each(|p| p.sort_by_desc(key));
    }

    fn sort_by_asc<K: Ord>(&mut self, key: impl FnMut(&DamageTablePart) -> K + Copy) {
        self.players.sort_unstable_by_key(key);

        self.players.iter_mut().for_each(|p| p.sort_by_asc(key));
    }
}

impl DamageTablePart {
    fn new(
        source: &DamageGroup,
        number_formatter: &mut NumberFormatter,
        id_source: &mut u32,
    ) -> Self {
        let id = *id_source;
        *id_source += 1;
        let sub_parts = source
            .sub_groups
            .values()
            .map(|s| DamageTablePart::new(s, number_formatter, id_source))
            .collect();
        Self {
            name: source.name.clone(),
            total_damage: ShieldAndHullTextValue::new(&source.total_damage, 2, number_formatter),
            dps: ShieldAndHullTextValue::new(&source.dps, 2, number_formatter),
            damage_percentage: TextValue::new(source.damage_percentage, 3, number_formatter),
            average_hit: ShieldAndHullTextValue::option(&source.average_hit, 2, number_formatter),
            critical_chance: TextValue::new(source.critical_chance, 3, number_formatter),
            flanking: TextValue::new(source.flanking, 3, number_formatter),
            max_one_hit: MaxOneHit::new(source, number_formatter),
            damage_resistance_percentage: ShieldAndHullTextValue::option(
                &source.damage_resistance_percentage,
                3,
                number_formatter,
            ),
            damage_resistance: ShieldAndHullTextValue::option(
                &source.damage_resistance,
                2,
                number_formatter,
            ),
            hits: Hits::new(source),
            sub_parts,
            open: false,
            id,
            source_hits: source
                .hits
                .iter()
                .chain(source.hits.iter())
                .copied()
                .collect(),
        }
    }

    pub fn dps(&self) -> f64 {
        self.dps.all.value.unwrap()
    }

    pub fn total_damage(&self) -> f64 {
        self.total_damage.all.value.unwrap()
    }

    fn show(
        &mut self,
        table: &mut TableBody,
        indent: f32,
        selected_id: &mut Option<u32>,
        on_selected: &mut impl FnMut(TableSelection),
    ) {
        table.row(ROW_HEIGHT, |mut r| {
            r.col(|ui| {
                ui.horizontal(|ui| {
                    ui.add_space(indent * 30.0);
                    let symbol = if self.open { "⏷" } else { "⏵" };
                    let can_open = self.sub_parts.len() > 0;
                    if ui
                        .add_visible(can_open, SelectableLabel::new(false, symbol))
                        .clicked()
                    {
                        self.open = !self.open;
                    }
                    let name_response =
                        ui.selectable_label(Some(self.id) == *selected_id, &self.name);
                    if name_response.clicked() {
                        if *selected_id == Some(self.id) {
                            *selected_id = None;
                            on_selected(TableSelection::Unselect);
                        } else {
                            *selected_id = Some(self.id);
                            on_selected(TableSelection::SubPartsOrSingle(self));
                        }
                    }

                    name_response.context_menu(|ui| {
                        if ui
                            .selectable_label(false, "copy name to clipboard")
                            .clicked()
                        {
                            ui.output().copied_text = self.name.clone();
                            ui.close_menu();
                        }

                        if ui
                            .selectable_label(false, "show diagrams for this")
                            .clicked()
                        {
                            *selected_id = Some(self.id);
                            on_selected(TableSelection::Single(self));
                            ui.close_menu();
                        }
                    });
                });
            });

            for column in COLUMNS.iter() {
                (column.show)(self, &mut r);
            }
        });

        if self.open {
            for sub_part in self.sub_parts.iter_mut() {
                sub_part.show(table, indent + 1.0, selected_id, on_selected);
            }
        }
    }

    fn sort_by_desc<K: Ord>(&mut self, mut key: impl FnMut(&DamageTablePart) -> K + Copy) {
        self.sub_parts.sort_unstable_by_key(|p| Reverse(key(p)));

        self.sub_parts.iter_mut().for_each(|p| p.sort_by_desc(key));
    }

    fn sort_by_asc<K: Ord>(&mut self, key: impl FnMut(&DamageTablePart) -> K + Copy) {
        self.sub_parts.sort_unstable_by_key(key);

        self.sub_parts.iter_mut().for_each(|p| p.sort_by_asc(key));
    }
}

impl MaxOneHit {
    fn new(source: &DamageGroup, number_formatter: &mut NumberFormatter) -> Self {
        Self {
            damage: TextValue::new(source.max_one_hit.damage, 2, number_formatter),
            name: source.max_one_hit.name.clone(),
        }
    }

    fn show(&self, row: &mut TableRow) {
        if let Some(response) = self.damage.show(row) {
            response.on_hover_text(&self.name);
        }
    }
}

impl Hits {
    fn new(source: &DamageGroup) -> Self {
        Self {
            all: source.damage_metrics.hits,
            all_text: source.damage_metrics.hits.to_string(),
            shield: source.shield_hits.to_string(),
            hull: source.hull_hits.to_string(),
        }
    }

    fn show(&self, row: &mut TableRow) {
        let response = row
            .col(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(&self.all_text);
                });
            })
            .1;

        show_shield_hull_values_tool_tip(response, &self.shield, &self.hull);
    }
}
