use arboard::Clipboard;
use bitflags::bitflags;
use eframe::egui::*;
use egui_extras::{Column, TableBody, TableBuilder, TableRow};

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
    name: String,
}

struct TextValue {
    text: String,
    value: f64,
}

impl DamageTable {
    pub fn empty() -> Self {
        Self {
            identifier: "<no data loaded>".to_string(),
            players: Vec::new(),
        }
    }

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
        TableBuilder::new(ui)
            .columns(Column::auto(), 7)
            .striped(true)
            .header(0.0, |mut r| {
                r.col(|ui| {
                    ui.label("Name");
                });
                self.show_column_header(&mut r, "Total Damage", TableColumns::TOTAL_DAMAGE);
                self.show_column_header(&mut r, "DPS", TableColumns::DPS);
                self.show_column_header(&mut r, "Max One-Hit", TableColumns::MAX_ONE_HIT);
                self.show_column_header(&mut r, "Average Hit", TableColumns::AVERAGE_HIT);
                self.show_column_header(&mut r, "Critical Chance %", TableColumns::CRITICAL_CHANCE);
                self.show_column_header(&mut r, "Flanking %", TableColumns::FLANKING);
            })
            .body(|mut t| {
                for player in self.players.iter_mut() {
                    player.show(&mut t, 0.0);
                }
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
    fn new(source: &DamageGroup, number_formatter: &mut NumberFormatter) -> Self {
        let max_one_hit = MaxOneHit {
            damage: TextValue::new(source.max_one_hit.damage, 2, number_formatter),
            name: source.max_one_hit.name.clone(),
        };
        let sub_parts = source
            .sub_groups
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

    fn show(&mut self, table: &mut TableBody, indent: f32) {
        table.row(20.0, |mut r| {
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
                    ui.label(&self.name).context_menu(|ui| {
                        if ui
                            .selectable_label(false, "copy name to clipboard")
                            .clicked()
                        {
                            if let Ok(mut clipboard) = Clipboard::new() {
                                _ = clipboard.set_text(&self.name);
                            }
                        }
                    });
                });
            });

            self.total_damage.show(&mut r);
            self.dps.show(&mut r);
            self.max_one_hit
                .damage
                .show(&mut r)
                .on_hover_text(&self.max_one_hit.name);
            self.average_hit.show(&mut r);
            self.critical_chance.show(&mut r);
            self.flanking.show(&mut r);
        });

        if self.open {
            for sub_part in self.sub_parts.iter_mut() {
                sub_part.show(table, indent + 1.0);
            }
        }
    }

    fn sort_by_key(&mut self, mut key: impl FnMut(&Self) -> f64 + Copy) {
        self.sub_parts
            .sort_unstable_by(|p1, p2| key(p1).total_cmp(&key(p2)).reverse());

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

    fn show(&self, row: &mut TableRow) -> Response {
        row.col(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(&self.text)
            });
        })
        .1
    }
}
