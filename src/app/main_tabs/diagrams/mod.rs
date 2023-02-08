mod common;
mod damage_chart;
mod damage_resistance_chart;
mod dps_graph;
mod summary_chart;

pub use common::PreparedDamageDataSet;
pub use dps_graph::DpsGraph;
use eframe::egui::Ui;
use itertools::Itertools;
pub use summary_chart::SummaryChart;

use crate::analyzer::DamageGroup;

use self::{damage_chart::DamageChart, damage_resistance_chart::*};

pub struct DamageDiagrams {
    dps_graph: DpsGraph,
    damage_chart: DamageChart,
    damage_resistance_chart: DamageResistanceChart,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ActiveDamageDiagram {
    Damage,
    Dps,
    DamageResistance,
}

impl DamageDiagrams {
    pub fn empty() -> Self {
        Self {
            dps_graph: DpsGraph::empty(),
            damage_chart: DamageChart::empty(),
            damage_resistance_chart: DamageResistanceChart::empty(),
        }
    }

    pub fn from_damage_groups<'a>(
        groups: impl Iterator<Item = &'a DamageGroup>,
        dps_filter: f64,
        damage_time_slice: f64,
    ) -> Self {
        let data = groups.map(|g| {
            PreparedDamageDataSet::new(
                g.name.as_str(),
                g.dps.all,
                g.total_damage.all,
                g.hits.iter(),
            )
        });

        Self::from_data(data, dps_filter, damage_time_slice)
    }

    pub fn from_data(
        data: impl Iterator<Item = PreparedDamageDataSet>,
        dps_filter: f64,
        damage_time_slice: f64,
    ) -> Self {
        let data = data.collect_vec();
        Self {
            dps_graph: DpsGraph::from_data(data.iter().cloned(), dps_filter),
            damage_chart: DamageChart::from_data(data.iter().cloned(), damage_time_slice),
            damage_resistance_chart: DamageResistanceChart::from_data(
                data.into_iter(),
                damage_time_slice,
            ),
        }
    }

    pub fn update(&mut self, dps_filter: f64, time_slice: f64) {
        self.dps_graph.update(dps_filter);
        self.damage_chart.update(time_slice);
        self.damage_resistance_chart.update(time_slice);
    }

    pub fn show(&mut self, ui: &mut Ui, active_diagram: ActiveDamageDiagram) {
        match active_diagram {
            ActiveDamageDiagram::Damage => self.damage_chart.show(ui),
            ActiveDamageDiagram::Dps => self.dps_graph.show(ui),
            ActiveDamageDiagram::DamageResistance => self.damage_resistance_chart.show(ui),
        }
    }
}

impl ActiveDamageDiagram {
    pub const fn display(&self) -> &'static str {
        match self {
            ActiveDamageDiagram::Damage => "Damage",
            ActiveDamageDiagram::Dps => "DPS",
            ActiveDamageDiagram::DamageResistance => "Damage Resistance",
        }
    }
}
