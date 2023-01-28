use eframe::egui::*;
use egui_extras::*;

use crate::{
    analyzer::Combat, custom_widgets::splitter::Splitter,
    helpers::number_formatting::NumberFormatter,
};

use super::{common::*, diagrams::SummaryChart, tables::SummaryTable};

pub struct SummaryTab {
    identifier: String,
    name: String,

    combat_duration: TextDuration,
    active_duration: TextDuration,
    total_damage_out: ShieldAndHullTextValue,
    total_damage_in: ShieldAndHullTextValue,
    total_kills: TextCount,
    total_deaths: TextCount,
    summary_table: SummaryTable,
    summary_dps_chart: SummaryChart,
    summary_damage_out_chart: SummaryChart,
    summary_damage_in_chart: SummaryChart,

    chart_tab: ChartTab,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum ChartTab {
    #[default]
    Dps,
    DamageOut,
    DamageIn,
}

impl SummaryTab {
    pub fn empty() -> Self {
        let nothing_loaded = "<no data loaded>".to_string();
        Self {
            identifier: nothing_loaded.clone(),
            name: nothing_loaded,
            summary_table: SummaryTable::empty(),
            combat_duration: Default::default(),
            active_duration: Default::default(),
            total_damage_out: Default::default(),
            total_damage_in: Default::default(),
            total_kills: Default::default(),
            total_deaths: Default::default(),
            summary_dps_chart: SummaryChart::empty(),
            summary_damage_out_chart: SummaryChart::empty(),
            summary_damage_in_chart: SummaryChart::empty(),
            chart_tab: Default::default(),
        }
    }

    pub fn update(&mut self, combat: &Combat) {
        self.identifier = combat.identifier();
        self.name = combat.name();

        self.combat_duration =
            TextDuration::new(time_range_to_duration_or_zero(&combat.combat_time));
        self.active_duration = TextDuration::new(time_range_to_duration(&combat.active_time));

        let mut number_formatter = NumberFormatter::new();
        self.total_damage_out =
            ShieldAndHullTextValue::new(&combat.total_damage_out, 2, &mut number_formatter);
        self.total_damage_in =
            ShieldAndHullTextValue::new(&combat.total_damage_in, 2, &mut number_formatter);
        self.total_kills = TextCount::new(combat.total_kills);
        self.total_deaths = TextCount::new(combat.total_deaths);

        self.summary_table = SummaryTable::new(combat);
        self.summary_dps_chart = SummaryChart::from_data(
            "summary dps chart",
            combat
                .players
                .values()
                .map(|p| (p.damage_out.name.as_str(), p.damage_out.dps.all)),
        );
        self.summary_damage_out_chart = SummaryChart::from_data(
            "summary damage in chart",
            combat
                .players
                .values()
                .map(|p| (p.damage_out.name.as_str(), p.damage_out.total_damage.all)),
        );
        self.summary_damage_in_chart = SummaryChart::from_data(
            "summary damage out chart",
            combat
                .players
                .values()
                .map(|p| (p.damage_out.name.as_str(), p.damage_in.total_damage.all)),
        );
    }

    pub fn show(&mut self, top_ui: &mut Ui) {
        top_ui.heading(&self.name);

        Splitter::horizontal()
            .initial_ratio(0.7)
            .show(top_ui, |top_ui, bottom_ui| {
                ScrollArea::both()
                    .min_scrolled_height(0.0)
                    .show(top_ui, |ui| {
                        ui.add_space(20.0);

                        ui.push_id("combat summary table", |ui| {
                            self.show_combat_summary_table(ui);
                        });

                        ui.add_space(20.0);

                        self.summary_table.show(ui);
                    });

                bottom_ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.chart_tab, ChartTab::Dps, "DPS");
                    ui.selectable_value(&mut self.chart_tab, ChartTab::DamageOut, "Damage Out");
                    ui.selectable_value(&mut self.chart_tab, ChartTab::DamageIn, "Damage In");
                });

                match self.chart_tab {
                    ChartTab::Dps => self.summary_dps_chart.show(bottom_ui),
                    ChartTab::DamageOut => self.summary_damage_out_chart.show(bottom_ui),
                    ChartTab::DamageIn => self.summary_damage_in_chart.show(bottom_ui),
                }
            });
    }

    fn show_combat_summary_table(&mut self, ui: &mut Ui) {
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
