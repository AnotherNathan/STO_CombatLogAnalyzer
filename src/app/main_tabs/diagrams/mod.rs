mod common;
mod damage_resistance_chart;
mod summary_chart;
mod value_per_second_graph;
mod values_chart;

pub use common::PreparedDamageDataSet;
pub use common::PreparedHealDataSet;
use eframe::egui::Ui;
use itertools::Itertools;
pub use summary_chart::SummaryChart;
pub use value_per_second_graph::ValuePerSecondGraph;

use crate::analyzer::*;

use self::{damage_resistance_chart::*, value_per_second_graph::*, values_chart::*};

pub struct DamageDiagrams {
    dps_graph: DpsGraph,
    damage_chart: DamageChart,
    damage_resistance_chart: DamageResistanceChart,
}

pub struct HealDiagrams {
    hps_graph: HpsGraph,
    heal_chart: HealChart,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ActiveDamageDiagram {
    Damage,
    Dps,
    DamageResistance,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ActiveHealDiagram {
    Heal,
    Hps,
}

impl DamageDiagrams {
    pub fn empty() -> Self {
        Self {
            dps_graph: ValuePerSecondGraph::empty(),
            damage_chart: ValuesChart::empty(),
            damage_resistance_chart: DamageResistanceChart::empty(),
        }
    }

    pub fn from_damage_groups<'a>(
        groups: impl Iterator<Item = &'a DamageGroup>,
        combat: &Combat,
        dps_filter: f64,
        damage_time_slice: f64,
    ) -> Self {
        let data = groups.map(|g| {
            PreparedDamageDataSet::new(
                g.name().get(&combat.name_manager),
                g.dps.all,
                g.total_damage.all,
                g.hits.get(&combat.hits_manger).iter(),
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

    pub fn add_data(&mut self, data: PreparedDamageDataSet, dps_filter: f64, time_slice: f64) {
        self.dps_graph.add_line(data.clone(), dps_filter);
        self.damage_chart.add_bars(data.clone(), time_slice);
        self.damage_resistance_chart.add_bars(data, time_slice);
    }

    pub fn remove_data(&mut self, data: &str) {
        self.dps_graph.remove_line(data);
        self.damage_chart.remove_bars(data);
        self.damage_resistance_chart.remove_bars(data);
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

impl HealDiagrams {
    pub fn empty() -> Self {
        Self {
            hps_graph: HpsGraph::empty(),
            heal_chart: HealChart::empty(),
        }
    }

    pub fn from_heal_groups<'a>(
        groups: impl Iterator<Item = &'a HealGroup>,
        combat: &Combat,
        dps_filter: f64,
        damage_time_slice: f64,
    ) -> Self {
        let data = groups.map(|g| {
            PreparedHealDataSet::new(
                g.name().get(&combat.name_manager),
                g.hps.all,
                g.total_heal.all,
                g.ticks.get(&combat.heal_ticks_manger).iter(),
            )
        });

        Self::from_data(data, dps_filter, damage_time_slice)
    }

    pub fn from_data(
        data: impl Iterator<Item = PreparedHealDataSet>,
        hps_filter: f64,
        heal_time_slice: f64,
    ) -> Self {
        let data = data.collect_vec();
        Self {
            hps_graph: HpsGraph::from_data(data.iter().cloned(), hps_filter),
            heal_chart: HealChart::from_data(data.iter().cloned(), heal_time_slice),
        }
    }

    pub fn add_data(&mut self, data: PreparedHealDataSet, hps_filter: f64, time_slice: f64) {
        self.hps_graph.add_line(data.clone(), hps_filter);
        self.heal_chart.add_bars(data.clone(), time_slice);
    }

    pub fn remove_data(&mut self, data: &str) {
        self.hps_graph.remove_line(data);
        self.heal_chart.remove_bars(data);
    }

    pub fn update(&mut self, hps_filter: f64, time_slice: f64) {
        self.hps_graph.update(hps_filter);
        self.heal_chart.update(time_slice);
    }

    pub fn show(&mut self, ui: &mut Ui, active_diagram: ActiveHealDiagram) {
        match active_diagram {
            ActiveHealDiagram::Heal => self.heal_chart.show(ui),
            ActiveHealDiagram::Hps => self.hps_graph.show(ui),
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

impl ActiveHealDiagram {
    pub const fn display(&self) -> &'static str {
        match self {
            ActiveHealDiagram::Heal => "Heal",
            ActiveHealDiagram::Hps => "HPS",
        }
    }
}
