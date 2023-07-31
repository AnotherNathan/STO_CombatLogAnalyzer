use std::cmp::Reverse;

use chrono::Duration;
use eframe::egui::*;

use crate::{
    analyzer::{Player as AnalyzedPlayer, *},
    app::main_tabs::common::*,
    custom_widgets::table::*,
    helpers::{number_formatting::NumberFormatter, *},
};

use super::common::Kills;

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
        "Outgoing DPS",
        |t| t.sort_by_option_f64(|p| p.dps_out.all.value),
        |p, r| p.dps_out.show(r),
    ),
    col!(
        "Total Outgoing Damage",
        |t| t.sort_by_option_f64(|p| p.total_out_damage.all.value),
        |p, r| p.total_out_damage.show(r),
    ),
    col!(
        "Outgoing Damage %",
        |t| t.sort_by_option_f64(|p| p.total_out_damage_percentage.all.value),
        |p, r| p.total_out_damage_percentage.show(r),
    ),
    col!(
        "Total Incoming Damage",
        |t| t.sort_by_option_f64(|p| p.total_in_damage.all.value),
        |p, r| p.total_in_damage.show(r),
    ),
    col!(
        "Incoming Damage %",
        |t| t.sort_by_option_f64(|p| p.total_in_damage_percentage.all.value),
        |p, r| p.total_in_damage_percentage.show(r),
    ),
    col!(
        "Combat Duration",
        |t| t.sort_by_key(|p| p.combat_duration.duration),
        |p, r| {
            p.combat_duration.show(r);
        },
    ),
    col!(
        "Combat Duration %",
        |t| t.sort_by_option_f64(|p| p.combat_duration_percentage.value),
        |p, r| {
            p.combat_duration.show(r);
        },
    ),
    col!(
        "Active Duration",
        |t| t.sort_by_key(|p| p.active_duration.duration),
        |p, r| {
            p.active_duration.show(r);
        },
    ),
    col!("Deaths", |t| t.sort_by_key(|p| p.deaths.count), |p, r| {
        p.deaths.show(r);
    }),
    col!(
        "Kills",
        |t| t.sort_by_key(|p| p.kills.total_count),
        |p, r| p.kills.show(r),
    ),
    col!(
        "Player Kills",
        |t| t.sort_by_key(|p| p.player_kills.count),
        |p, r| {
            p.player_kills.show(r);
        },
    ),
    col!(
        "NPC Kills",
        |t| t.sort_by_key(|p| p.npc_kills.count),
        |p, r| {
            p.npc_kills.show(r);
        },
    ),
];

struct ColumnDescriptor {
    name: &'static str,
    sort: fn(&mut SummaryTable),
    show: fn(&Player, &mut TableRow),
}

pub struct SummaryTable {
    players: Vec<Player>,
    selected_player: Option<usize>,
}

struct Player {
    name: String,
    total_out_damage: ShieldAndHullTextValue,
    dps_out: ShieldAndHullTextValue,
    total_out_damage_percentage: ShieldAndHullTextValue,
    total_in_damage: ShieldAndHullTextValue,
    total_in_damage_percentage: ShieldAndHullTextValue,
    combat_duration: TextDuration,
    combat_duration_percentage: TextValue,
    active_duration: TextDuration,
    kills: Kills,
    npc_kills: TextCount,
    player_kills: TextCount,
    deaths: TextCount,
}

impl SummaryTable {
    pub fn empty() -> Self {
        Self {
            players: Default::default(),
            selected_player: None,
        }
    }

    pub fn new(combat: &Combat) -> Self {
        let combat_duration = time_range_to_duration_or_zero(&combat.combat_time);
        let mut number_formatter = NumberFormatter::new();
        let mut table = Self {
            players: combat
                .players
                .values()
                .map(|p| {
                    Player::new(
                        combat_duration,
                        p,
                        &combat.name_manager,
                        &mut number_formatter,
                    )
                })
                .collect(),
            selected_player: None,
        };
        table.sort_by_option_f64(|p| p.total_out_damage.all.value);
        table
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ScrollArea::new([true, false]).show(ui, |ui| {
            Table::new(ui)
                .header(HEADER_HEIGHT, |r| {
                    r.cell(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Player");
                        });
                    });

                    for column in COLUMNS.iter() {
                        Self::show_column_header(r, column.name, || {
                            (column.sort)(self);
                        });
                    }
                })
                .body(ROW_HEIGHT, |t| {
                    for (i, player) in self.players.iter().enumerate() {
                        let player_selected = Some(i) == self.selected_player;
                        if player.show(t, player_selected).clicked() {
                            self.selected_player = if player_selected { None } else { Some(i) };
                        }
                    }
                });
        });
    }

    fn show_column_header(row: &mut TableRow, column_name: &str, sort: impl FnOnce()) {
        if row
            .selectable_cell(false, |ui| {
                ui.label(column_name);
            })
            .clicked()
        {
            sort();
        }
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
        name_manager: &NameManager,
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
        let npc_kills: u32 = player
            .damage_out
            .kills
            .iter()
            .filter_map(|(n, k)| {
                if !name_manager.info(*n).flags.contains(NameFlags::PLAYER) {
                    Some(*k)
                } else {
                    None
                }
            })
            .sum();
        let player_kills: u32 = player
            .damage_out
            .kills
            .iter()
            .filter_map(|(n, k)| {
                if name_manager.info(*n).flags.contains(NameFlags::PLAYER) {
                    Some(*k)
                } else {
                    None
                }
            })
            .sum();
        Self {
            name: player.damage_out.name().get(name_manager).to_string(),
            total_out_damage: ShieldAndHullTextValue::new(
                &player.damage_out.total_damage,
                2,
                number_formatter,
            ),
            total_out_damage_percentage: ShieldAndHullTextValue::option(
                &player.damage_out.damage_percentage,
                3,
                number_formatter,
            ),
            dps_out: ShieldAndHullTextValue::new(&player.damage_out.dps, 2, number_formatter),
            total_in_damage: ShieldAndHullTextValue::new(
                &player.damage_in.total_damage,
                2,
                number_formatter,
            ),
            total_in_damage_percentage: ShieldAndHullTextValue::option(
                &player.damage_in.damage_percentage,
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
            kills: Kills::new(&player.damage_out, name_manager),
            deaths: TextCount::new(player.damage_in.kills.values().copied().sum::<u32>() as _),
            npc_kills: TextCount::new(npc_kills as _),
            player_kills: TextCount::new(player_kills as _),
        }
    }

    pub fn show(&self, table: &mut TableBody, selected: bool) -> Response {
        table.selectable_row(selected, |r| {
            r.cell(|ui| {
                ui.label(&self.name);
            });

            for column in COLUMNS.iter() {
                (column.show)(self, r);
            }
        })
    }
}
