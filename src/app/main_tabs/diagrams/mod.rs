mod common;
mod damage_chart;
mod dps_graph;

pub use common::PreparedDataSet;
pub use dps_graph::DpsGraph;
use eframe::egui::Ui;
use itertools::Itertools;

use crate::analyzer::DamageGroup;

use self::damage_chart::DamageChart;

pub struct DamageDiagrams {
    dps_graph: DpsGraph,
    damage_chart: DamageChart,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ActiveDamageDiagram {
    Damage,
    Dps,
}

impl DamageDiagrams {
    pub fn empty() -> Self {
        Self {
            dps_graph: DpsGraph::empty(),
            damage_chart: DamageChart::empty(),
        }
    }

    pub fn from_damage_groups<'a>(
        groups: impl Iterator<Item = &'a DamageGroup>,
        dps_filter: f64,
        damage_time_slice: f64,
    ) -> Self {
        let data = groups.map(|g| {
            PreparedDataSet::new(
                g.name.as_str(),
                g.hull_hits.iter().chain(g.shield_hits.iter()),
            )
        });

        Self::from_data(data, dps_filter, damage_time_slice)
    }

    pub fn from_data(
        data: impl Iterator<Item = PreparedDataSet>,
        dps_filter: f64,
        damage_time_slice: f64,
    ) -> Self {
        let data = data.collect_vec();
        Self {
            dps_graph: DpsGraph::from_data(data.iter().cloned(), dps_filter),
            damage_chart: DamageChart::from_data(data.into_iter(), damage_time_slice),
        }
    }

    pub fn update(&mut self, dps_filter: f64, damage_time_slice: f64) {
        self.dps_graph.update(dps_filter);
        self.damage_chart.update(damage_time_slice)
    }

    pub fn show(&mut self, ui: &mut Ui, active_diagram: ActiveDamageDiagram) {
        match active_diagram {
            ActiveDamageDiagram::Damage => self.damage_chart.show(ui),
            ActiveDamageDiagram::Dps => self.dps_graph.show(ui),
        }
    }
}

impl ActiveDamageDiagram {
    pub const fn display(&self) -> &'static str {
        match self {
            ActiveDamageDiagram::Damage => "Damage",
            ActiveDamageDiagram::Dps => "DPS",
        }
    }
}
