use eframe::egui::*;
use egui_extras::*;

use crate::{analyzer::Combat, helpers::number_formatting::NumberFormatter};

use self::{common::*, damage_table::DamageTable, summary_table::SummaryTable};

mod common;
pub mod damage_table;
pub mod summary_table;

pub struct MainTabs {
    pub identifier: String,
    pub name: String,

    pub combat_duration: TextDuration,
    pub active_duration: TextDuration,
    pub total_damage_out: ShieldAndHullTextValue,
    pub total_damage_in: ShieldAndHullTextValue,
    pub total_kills: TextCount,
    pub total_deaths: TextCount,

    pub summary_table: SummaryTable,
    pub damage_out_table: DamageTable,
    pub damage_in_table: DamageTable,

    active_tab: MainTab,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MainTab {
    #[default]
    Summary,
    DamageOut,
    DamageIn,
}

impl MainTabs {
    pub fn empty() -> Self {
        let nothing_loaded = "<no data loaded>".to_string();
        Self {
            identifier: nothing_loaded.clone(),
            name: nothing_loaded,
            summary_table: SummaryTable::empty(),
            damage_out_table: DamageTable::empty(),
            damage_in_table: DamageTable::empty(),
            active_tab: Default::default(),
            combat_duration: Default::default(),
            active_duration: Default::default(),
            total_damage_out: Default::default(),
            total_damage_in: Default::default(),
            total_kills: Default::default(),
            total_deaths: Default::default(),
        }
    }

    pub fn update(&mut self, combat: &Combat) {
        self.identifier = combat.identifier();
        self.name = combat.name();

        self.combat_duration =
            TextDuration::new(time_range_to_duration_or_zero(&combat.combat_time));
        self.active_duration = TextDuration::new(time_range_to_duration(&combat.active_time));

        let mut number_formatter = NumberFormatter::new();
        self.total_damage_out = ShieldAndHullTextValue::new(
            combat.total_damage_out.all,
            combat.total_damage_out.shield,
            combat.total_damage_out.hull,
            2,
            &mut number_formatter,
        );
        self.total_damage_in = ShieldAndHullTextValue::new(
            combat.total_damage_in.all,
            combat.total_damage_in.shield,
            combat.total_damage_in.hull,
            2,
            &mut number_formatter,
        );
        self.total_kills = TextCount::new(combat.total_kills);
        self.total_deaths = TextCount::new(combat.total_deaths);

        self.summary_table = SummaryTable::new(combat);
        self.damage_out_table = DamageTable::new(combat, |p| &p.damage_out);
        self.damage_in_table = DamageTable::new(combat, |p| &p.damage_in);
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.active_tab, MainTab::Summary, "Summary");

            ui.selectable_value(&mut self.active_tab, MainTab::DamageOut, "Outgoing Damage");

            ui.selectable_value(&mut self.active_tab, MainTab::DamageIn, "Incoming Damage");
        });

        match self.active_tab {
            MainTab::Summary => self.show_summary_tab(ui),
            MainTab::DamageOut => self.damage_out_table.show(ui),
            MainTab::DamageIn => self.damage_in_table.show(ui),
        }
    }

    fn show_summary_tab(&mut self, ui: &mut Ui) {
        ui.heading(&self.name);

        ui.add_space(20.0);

        ui.push_id("combat summary table", |ui| {
            TableBuilder::new(ui)
                .columns(Column::auto(), 6)
                .striped(true)
                .max_scroll_height(f32::MAX)
                .body(|mut t| {
                    Self::summary_row(&mut t, "Combat Duration", &self.combat_duration.text);
                    Self::summary_row(
                        &mut t,
                        "Active Duration (duration of everything)",
                        &self.active_duration.text,
                    );

                    let response = Self::summary_row(
                        &mut t,
                        "Total Outgoing Damage",
                        &self.total_damage_out.all.text,
                    );
                    show_shield_hull_values_tool_tip(
                        response,
                        &self.total_damage_out.shield,
                        &self.total_damage_out.hull,
                    );

                    let response = Self::summary_row(
                        &mut t,
                        "Total Incoming Damage",
                        &self.total_damage_in.all.text,
                    );
                    show_shield_hull_values_tool_tip(
                        response,
                        &self.total_damage_in.shield,
                        &self.total_damage_in.hull,
                    );

                    Self::summary_row(&mut t, "Total Kills", &self.total_kills.text);
                    Self::summary_row(&mut t, "Total Deaths", &self.total_deaths.text);
                });
        });

        ui.add_space(20.0);

        self.summary_table.show(ui);
    }

    fn summary_row(table: &mut TableBody, description: &str, value: &str) -> Response {
        let mut response = None;
        table.row(ROW_HEIGHT, |mut r| {
            r.col(|ui| {
                ui.horizontal(|ui| {
                    ui.label(description);
                });
            });

            response = Some(
                r.col(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(value);
                    });
                })
                .1,
            );
        });

        response.unwrap()
    }
}
