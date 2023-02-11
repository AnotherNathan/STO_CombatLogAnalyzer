use std::cmp::Reverse;

use chrono::Duration;
use eframe::egui::*;
use egui_extras::*;

use crate::{
    analyzer::{Combat, Player as AnalyzedPlayer},
    app::main_tabs::common::*,
    helpers::{number_formatting::NumberFormatter, *},
};

pub struct SummaryTable {
    players: Vec<Player>,
}

struct Player {
    name: String,
    total_out_damage: ShieldAndHullTextValue,
    dps_out: ShieldAndHullTextValue,
    total_out_damage_percentage: TextValue,
    total_in_damage: ShieldAndHullTextValue,
    total_in_damage_percentage: TextValue,
    combat_duration: TextDuration,
    combat_duration_percentage: TextValue,
    active_duration: TextDuration,
    kills: TextCount,
    deaths: TextCount,
}

impl SummaryTable {
    pub fn empty() -> Self {
        Self {
            players: Default::default(),
        }
    }

    pub fn new(combat: &Combat) -> Self {
        let combat_duration = time_range_to_duration_or_zero(&combat.combat_time);
        let mut number_formatter = NumberFormatter::new();
        let mut table = Self {
            players: combat
                .players
                .values()
                .map(|p| Player::new(combat_duration, p, &mut number_formatter))
                .collect(),
        };
        table.sort_by_option_f64(|p| p.total_out_damage.all.value);
        table
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ScrollArea::new([true, false]).show(ui, |ui| {
            TableBuilder::new(ui)
                .columns(Column::auto(), 11)
                .striped(true)
                .max_scroll_height(f32::MAX)
                .header(0.0, |mut r| {
                    r.col(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Player");
                        });
                    });
                    Self::show_column_header(&mut r, "Total Outgoing Damage", || {
                        self.sort_by_option_f64(|p| p.total_out_damage.all.value)
                    });

                    Self::show_column_header(&mut r, "Outgoing DPS", || {
                        self.sort_by_option_f64(|p| p.dps_out.all.value)
                    });

                    Self::show_column_header(&mut r, "Outgoing Damage %", || {
                        self.sort_by_option_f64(|p| p.total_out_damage_percentage.value)
                    });

                    Self::show_column_header(&mut r, "Total Incoming Damage", || {
                        self.sort_by_option_f64(|p| p.total_in_damage.all.value)
                    });

                    Self::show_column_header(&mut r, "Incoming Damage %", || {
                        self.sort_by_option_f64(|p| p.total_in_damage_percentage.value)
                    });

                    Self::show_column_header(&mut r, "Combat Duration", || {
                        self.sort_by_key(|p| p.combat_duration.duration)
                    });

                    Self::show_column_header(&mut r, "Combat Duration %", || {
                        self.sort_by_option_f64(|p| p.combat_duration_percentage.value)
                    });

                    Self::show_column_header(&mut r, "Active Duration", || {
                        self.sort_by_key(|p| p.active_duration.duration)
                    });

                    Self::show_column_header(&mut r, "Kills", || {
                        self.sort_by_key(|p| p.kills.count)
                    });

                    Self::show_column_header(&mut r, "Deaths", || {
                        self.sort_by_key(|p| p.deaths.count)
                    });
                })
                .body(|mut t| {
                    for player in self.players.iter() {
                        player.show(&mut t)
                    }
                });
        });
    }

    fn show_column_header(row: &mut TableRow, column_name: &str, sort: impl FnOnce()) {
        row.col(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.selectable_label(false, column_name).clicked() {
                    sort();
                }
            });
        });
    }

    fn sort_by_option_f64(&mut self, mut value: impl FnMut(&Player) -> Option<f64>) {
        self.players
            .sort_unstable_by_key(|p| Reverse(value(p).map(|v| F64TotalOrd(v))))
    }

    fn sort_by_key<K: Ord>(&mut self, mut key: impl FnMut(&Player) -> K) {
        self.players.sort_unstable_by_key(|p| Reverse(key(p)));
    }
}

impl Player {
    fn new(
        combat_duration: Duration,
        player: &AnalyzedPlayer,
        number_formatter: &mut NumberFormatter,
    ) -> Self {
        let player_combat_duration = time_range_to_duration_or_zero(&player.combat_time);
        let player_combat_duration_percentage = if combat_duration.num_milliseconds() == 0 {
            0.0
        } else {
            player_combat_duration.num_milliseconds() as f64
                / combat_duration.num_milliseconds() as f64
                * 100.0
        };
        let player_active_duration = time_range_to_duration_or_zero(&player.active_time);
        Self {
            name: player.damage_out.name.clone(),
            total_out_damage: ShieldAndHullTextValue::new(
                &player.damage_out.total_damage,
                2,
                number_formatter,
            ),
            total_out_damage_percentage: TextValue::new(
                player.damage_out.damage_percentage,
                3,
                number_formatter,
            ),
            dps_out: ShieldAndHullTextValue::new(&player.damage_out.dps, 2, number_formatter),
            total_in_damage: ShieldAndHullTextValue::new(
                &player.damage_in.total_damage,
                2,
                number_formatter,
            ),
            total_in_damage_percentage: TextValue::new(
                player.damage_in.damage_percentage,
                3,
                number_formatter,
            ),
            combat_duration: TextDuration::new(player_combat_duration),
            combat_duration_percentage: TextValue::new(
                player_combat_duration_percentage,
                3,
                number_formatter,
            ),
            active_duration: TextDuration::new(player_active_duration),
            kills: TextCount::new(player.kills),
            deaths: TextCount::new(player.deaths),
        }
    }

    pub fn show(&self, table: &mut TableBody) {
        table.row(ROW_HEIGHT, |mut r| {
            r.col(|ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.name);
                });
            });

            self.total_out_damage.show(&mut r);
            self.dps_out.show(&mut r);
            self.total_out_damage_percentage.show(&mut r);
            self.total_in_damage.show(&mut r);
            self.total_in_damage_percentage.show(&mut r);
            self.combat_duration.show(&mut r);
            self.combat_duration_percentage.show(&mut r);
            self.active_duration.show(&mut r);
            self.kills.show(&mut r);
            self.deaths.show(&mut r);
        })
    }
}
