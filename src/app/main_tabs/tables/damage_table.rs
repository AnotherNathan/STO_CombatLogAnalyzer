use bitflags::bitflags;
use eframe::egui::*;
use egui_extras::{Column, TableBody, TableBuilder, TableRow};

use crate::{analyzer::*, app::main_tabs::common::*, helpers::number_formatting::NumberFormatter};

pub struct DamageTable {
    players: Vec<DamageTablePart>,
    selected_id: Option<u32>,
}

bitflags! {
    pub struct TableColumns: u32{
        const NONE = 0;
        const TOTAL_DAMAGE = 1<<0;
        const DPS = 1<<1;
        const MAX_ONE_HIT = 1<<2;
        const AVERAGE_HIT = 1<<3;
        const CRITICAL_CHANCE = 1<<4;
        const FLANKING = 1<<5;
        const HITS = 1<<6;
        const DAMAGE_PERCENTAGE = 1<<7;
    }
}

pub struct DamageTablePart {
    pub name: String,
    total_damage: ShieldAndHullTextValue,
    dps: ShieldAndHullTextValue,
    damage_percentage: TextValue,
    max_one_hit: MaxOneHit,
    average_hit: TextValue,
    critical_chance: TextValue,
    flanking: TextValue,
    hits: Hits,
    pub sub_parts: Vec<DamageTablePart>,

    pub source_hits: Vec<Hit>,

    id: u32,

    open: bool,
}

struct MaxOneHit {
    damage: TextValue,
    name: String,
}

struct Hits {
    all: usize,
    all_text: String,
    shield: String,
    hull: String,
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
        table.sort(TableColumns::TOTAL_DAMAGE);

        table
    }

    pub fn show(&mut self, ui: &mut Ui, mut on_selected: impl FnMut(Option<&DamageTablePart>)) {
        ScrollArea::horizontal()
            .min_scrolled_width(0.0)
            .show(ui, |ui| {
                TableBuilder::new(ui)
                    .columns(Column::auto(), 9)
                    .striped(true)
                    .min_scrolled_height(0.0)
                    .max_scroll_height(f32::MAX)
                    .header(0.0, |mut r| {
                        r.col(|ui| {
                            ui.label("Name");
                        });
                        self.show_column_header(&mut r, "Total Damage", TableColumns::TOTAL_DAMAGE);
                        self.show_column_header(&mut r, "DPS", TableColumns::DPS);
                        self.show_column_header(&mut r, "Damage %", TableColumns::DPS);
                        self.show_column_header(&mut r, "Max One-Hit", TableColumns::MAX_ONE_HIT);
                        self.show_column_header(&mut r, "Average Hit", TableColumns::AVERAGE_HIT);
                        self.show_column_header(
                            &mut r,
                            "Critical Chance %",
                            TableColumns::CRITICAL_CHANCE,
                        );
                        self.show_column_header(&mut r, "Flanking %", TableColumns::FLANKING);
                        self.show_column_header(&mut r, "Hits", TableColumns::HITS);
                    })
                    .body(|mut t| {
                        for player in self.players.iter_mut() {
                            player.show(&mut t, 0.0, &mut self.selected_id, &mut on_selected);
                        }
                    });
            });
    }

    fn show_column_header(&mut self, row: &mut TableRow, column_name: &str, column: TableColumns) {
        row.col(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.selectable_label(false, column_name).clicked() {
                    self.sort(column);
                }
            });
        });
    }

    pub fn sort(&mut self, by_column: TableColumns) {
        if by_column.contains(TableColumns::TOTAL_DAMAGE) {
            self.sort_by(|p| p.total_damage.all.value);
        } else if by_column.contains(TableColumns::DPS) {
            self.sort_by(|p| p.dps.all.value);
        } else if by_column.contains(TableColumns::MAX_ONE_HIT) {
            self.sort_by(|p| p.max_one_hit.damage.value);
        } else if by_column.contains(TableColumns::AVERAGE_HIT) {
            self.sort_by(|p| p.average_hit.value);
        } else if by_column.contains(TableColumns::CRITICAL_CHANCE) {
            self.sort_by(|p| p.critical_chance.value);
        } else if by_column.contains(TableColumns::FLANKING) {
            self.sort_by(|p| p.flanking.value);
        } else if by_column.contains(TableColumns::HITS) {
            self.sort_by_key(|p| p.hits.all);
        } else if by_column.contains(TableColumns::DAMAGE_PERCENTAGE) {
            self.sort_by(|p| p.damage_percentage.value);
        }
    }

    fn sort_by(&mut self, mut key: impl FnMut(&DamageTablePart) -> f64 + Copy) {
        self.players
            .sort_unstable_by(|p1, p2| key(p1).total_cmp(&key(p2)).reverse());

        self.players.iter_mut().for_each(|p| p.sort_by(key));
    }

    fn sort_by_key<K: Ord>(&mut self, mut key: impl FnMut(&DamageTablePart) -> K + Copy) {
        self.players.sort_unstable_by_key(|p| key(p));

        self.players.iter_mut().for_each(|p| p.sort_by_key(key));
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
            total_damage: ShieldAndHullTextValue::new(
                source.total_damage.all,
                source.total_damage.shield,
                source.total_damage.hull,
                2,
                number_formatter,
            ),
            dps: ShieldAndHullTextValue::new(
                source.dps,
                source.shield_dps,
                source.hull_dps,
                2,
                number_formatter,
            ),
            damage_percentage: TextValue::new(source.damage_percentage, 3, number_formatter),
            average_hit: TextValue::new(source.average_hit, 2, number_formatter),
            critical_chance: TextValue::new(source.critical_chance, 3, number_formatter),
            flanking: TextValue::new(source.flanking, 3, number_formatter),
            max_one_hit: MaxOneHit::new(source, number_formatter),
            hits: Hits::new(source),
            sub_parts,
            open: false,
            id,
            source_hits: source
                .hull_hits
                .iter()
                .chain(source.hull_hits.iter())
                .copied()
                .collect(),
        }
    }

    fn show(
        &mut self,
        table: &mut TableBody,
        indent: f32,
        selected_id: &mut Option<u32>,
        on_selected: &mut impl FnMut(Option<&DamageTablePart>),
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
                            on_selected(None);
                        } else {
                            *selected_id = Some(self.id);
                            on_selected(Some(self));
                        }
                    }
                    name_response.context_menu(|ui| {
                        if ui
                            .selectable_label(false, "copy name to clipboard")
                            .clicked()
                        {
                            ui.output().copied_text = self.name.clone();
                        }
                    });
                });
            });

            self.total_damage.show(&mut r);
            self.dps.show(&mut r);
            self.damage_percentage.show(&mut r);
            self.max_one_hit.show(&mut r);
            self.average_hit.show(&mut r);
            self.critical_chance.show(&mut r);
            self.flanking.show(&mut r);
            self.hits.show(&mut r);
        });

        if self.open {
            for sub_part in self.sub_parts.iter_mut() {
                sub_part.show(table, indent + 1.0, selected_id, on_selected);
            }
        }
    }

    fn sort_by(&mut self, mut key: impl FnMut(&Self) -> f64 + Copy) {
        self.sub_parts
            .sort_unstable_by(|p1, p2| key(p1).total_cmp(&key(p2)).reverse());

        self.sub_parts.iter_mut().for_each(|p| p.sort_by(key));
    }

    fn sort_by_key<K: Ord>(&mut self, mut key: impl FnMut(&DamageTablePart) -> K + Copy) {
        self.sub_parts.sort_unstable_by_key(|p| key(p));

        self.sub_parts.iter_mut().for_each(|p| p.sort_by_key(key));
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
        self.damage.show(row).on_hover_text(&self.name);
    }
}

impl Hits {
    fn new(source: &DamageGroup) -> Self {
        Self {
            all: source.hits(),
            all_text: source.hits().to_string(),
            shield: source.shield_hits().to_string(),
            hull: source.hull_hits().to_string(),
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
