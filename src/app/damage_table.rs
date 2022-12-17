use bitflags::bitflags;
use eframe::egui::*;

use crate::{analyzer::*, helpers::number_formatting::NumberFormatter};
use std::fmt::Write;

pub struct DamageTable {
    pub identifier: String,
    players: Vec<TablePart>,
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
    }
}

struct TablePart {
    name: String,
    total_damage: TextValue,
    dps: TextValue,
    max_one_hit: MaxOneHit,
    average_hit: TextValue,
    critical_chance: TextValue,
    flanking: TextValue,
    sub_parts: Vec<TablePart>,

    open: bool,
}

struct MaxOneHit {
    damage: TextValue,
    source: String,
}

struct TextValue {
    text: String,
    value: f64,
}

impl DamageTable {
    pub fn new(combat: &Combat) -> Self {
        let mut number_formatter = NumberFormatter::new();
        let mut table = Self {
            identifier: combat.identifier.clone(),
            players: combat
                .players
                .values()
                .map(|p| TablePart::new(&p.damage_source, &mut number_formatter))
                .collect(),
        };
        table.sort(TableColumns::TOTAL_DAMAGE);

        table
    }

    pub fn show(&mut self, ui: &mut Ui) {
        Grid::new(&self.identifier)
            .striped(true)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                ui.label("Name");
                self.show_column_header(ui, "Total Damage", TableColumns::TOTAL_DAMAGE);
                self.show_column_header(ui, "DPS", TableColumns::DPS);
                self.show_column_header(ui, "Max One-Hit", TableColumns::MAX_ONE_HIT);
                self.show_column_header(ui, "Average Hit", TableColumns::AVERAGE_HIT);
                self.show_column_header(ui, "Critical Chance %", TableColumns::CRITICAL_CHANCE);
                self.show_column_header(ui, "Flanking %", TableColumns::FLANKING);
                ui.end_row();

                for player in self.players.iter_mut() {
                    player.show(ui, 0.0);
                }
            });
    }

    fn show_column_header(&mut self, ui: &mut Ui, column_name: &str, column: TableColumns) {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.selectable_label(false, column_name).clicked() {
                self.sort(column);
            }
        });
    }

    pub fn sort(&mut self, by_column: TableColumns) {
        if by_column.contains(TableColumns::TOTAL_DAMAGE) {
            self.sort_by_key(|p| p.total_damage.value);
        } else if by_column.contains(TableColumns::DPS) {
            self.sort_by_key(|p| p.dps.value);
        } else if by_column.contains(TableColumns::MAX_ONE_HIT) {
            self.sort_by_key(|p| p.max_one_hit.damage.value);
        } else if by_column.contains(TableColumns::AVERAGE_HIT) {
            self.sort_by_key(|p| p.average_hit.value);
        } else if by_column.contains(TableColumns::CRITICAL_CHANCE) {
            self.sort_by_key(|p| p.critical_chance.value);
        } else if by_column.contains(TableColumns::FLANKING) {
            self.sort_by_key(|p| p.flanking.value);
        }
    }

    fn sort_by_key(&mut self, mut key: impl FnMut(&TablePart) -> f64 + Copy) {
        self.players
            .sort_unstable_by(|p1, p2| key(p1).total_cmp(&key(p2)).reverse());

        self.players.iter_mut().for_each(|p| p.sort_by_key(key));
    }
}

impl TablePart {
    fn new(source: &DamageSource, number_formatter: &mut NumberFormatter) -> Self {
        let max_one_hit = MaxOneHit {
            damage: TextValue::new(source.max_one_hit.hit.damage, 2, number_formatter),
            source: source.max_one_hit.source.clone(),
        };
        let sub_parts = source
            .sub_sources
            .values()
            .map(|s| TablePart::new(s, number_formatter))
            .collect();
        Self {
            name: source.name.clone(),
            total_damage: TextValue::new(source.total_damage, 2, number_formatter),
            dps: TextValue::new(source.dps, 2, number_formatter),
            average_hit: TextValue::new(source.average_hit, 2, number_formatter),
            critical_chance: TextValue::new(source.critical_chance, 3, number_formatter),
            flanking: TextValue::new(source.flanking, 3, number_formatter),
            max_one_hit,
            sub_parts,
            open: false,
        }
    }

    fn show(&mut self, ui: &mut Ui, indent: f32) {
        ui.horizontal(|ui| {
            ui.add_space(indent * 50.0);
            let symbol = if self.open { "⏷" } else { "⏵" };
            if self.sub_parts.len() > 0 && ui.selectable_label(false, symbol).clicked() {
                self.open = !self.open;
            }
            ui.label(&self.name);
        });

        self.total_damage.show(ui);
        self.dps.show(ui);
        self.max_one_hit.damage.show(ui);
        self.average_hit.show(ui);
        self.critical_chance.show(ui);
        self.flanking.show(ui);

        ui.end_row();

        if self.open {
            for sub_part in self.sub_parts.iter_mut() {
                sub_part.show(ui, indent + 1.0);
            }
        }
    }

    fn sort_by_key(&mut self, mut key: impl FnMut(&Self) -> f64 + Copy) {
        self.sub_parts
            .sort_unstable_by(|p1, p2| key(p1).total_cmp(&key(p2)));

        self.sub_parts.iter_mut().for_each(|p| p.sort_by_key(key));
    }
}

impl TextValue {
    fn new(value: f64, precision: usize, number_formatter: &mut NumberFormatter) -> Self {
        Self {
            text: number_formatter.format(value, precision),
            value,
        }
    }

    fn show(&self, ui: &mut Ui) -> Response {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(&self.text)
        })
        .inner
    }
}
