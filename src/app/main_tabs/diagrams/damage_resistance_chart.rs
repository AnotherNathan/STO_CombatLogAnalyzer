use std::ops::RangeInclusive;

use eframe::egui::*;
use egui_plot::*;
use itertools::Itertools;

use crate::{analyzer::*, helpers::number_formatting::NumberFormatter};

use super::common::*;

pub struct DamageResistanceChart {
    newly_created: bool,
    bars: Vec<DamageResistanceBars>,
    updated_time_slice: Option<f64>,
}

struct DamageResistanceBars {
    data: PreparedDamageDataSet,
    bars: Vec<Bar>,
}

impl DamageResistanceChart {
    pub fn empty() -> Self {
        Self {
            newly_created: true,
            bars: Vec::new(),
            updated_time_slice: None,
        }
    }

    pub fn from_data(bars: impl Iterator<Item = PreparedDamageDataSet>, time_slice: f64) -> Self {
        let bars: Vec<_> = bars.map(|d| DamageResistanceBars::new(d)).collect();
        Self {
            newly_created: true,
            bars,
            updated_time_slice: Some(time_slice),
        }
    }

    pub fn add_bars(&mut self, bars: PreparedDamageDataSet, time_slice: f64) {
        self.bars.push(DamageResistanceBars::new(bars));
        self.update(time_slice);
    }

    pub fn remove_bars(&mut self, bars: &str) {
        if let Some((index, _)) = self.bars.iter().find_position(|b| b.data.name == bars) {
            self.bars.remove(index);
        }
    }

    pub fn update(&mut self, time_slice: f64) {
        self.updated_time_slice = Some(time_slice);
    }

    pub fn show(&mut self, ui: &mut Ui) {
        if let Some(time_slice) = self.updated_time_slice.take() {
            self.bars.iter_mut().for_each(|b| b.update(time_slice));
        }

        let mut plot = Plot::new("damage resistance chart")
            .auto_bounds(true)
            .y_axis_formatter(Self::format_axis)
            .x_axis_formatter(Self::format_axis)
            .label_formatter(|_, p| {
                let mut formatter = NumberFormatter::new();
                format!(
                    "Resistance: {}%\nTime: {}",
                    formatter.format(p.y, 2),
                    formatter.format(p.x, 2)
                )
            })
            .legend(Legend::default());

        if self.newly_created {
            plot = plot.reset();
            self.newly_created = false;
        }

        if self.bars.len() == 0 {
            plot = plot.include_x(60.0);
        }

        plot.show(ui, |p| {
            for bars in self.bars.iter() {
                p.bar_chart(bars.chart());
            }
        });
    }

    fn format_axis(mark: GridMark, _: &RangeInclusive<f64>) -> String {
        let mut formatter = NumberFormatter::new();
        formatter.format(mark.value, 0)
    }
}

impl DamageResistanceBars {
    fn new(data: PreparedDamageDataSet) -> Self {
        Self {
            data,
            bars: Vec::new(),
        }
    }

    fn update(&mut self, time_slice: f64) {
        let bars = time_slices(&self.data, time_slice)
            .filter_map(|(time, s)| {
                let (damage, shield_damage, hull_damage, drain_damage, base_damage) =
                    s.iter().fold(
                        Default::default(),
                        |(damage, shield_damage, hull_damage, drain_damage, base_damage), h| {
                            (
                                damage + h.damage,
                                shield_damage + h.shield_damage,
                                hull_damage + h.hull_damage,
                                drain_damage + h.drain_damage,
                                base_damage + h.base_damage,
                            )
                        },
                    );

                let total_damage = &ShieldHullValues {
                    all: damage,
                    shield: shield_damage,
                    hull: hull_damage,
                };
                let resistance =
                    damage_resistance_percentage(&total_damage, base_damage, drain_damage)?;

                Some(
                    Bar::new(time, resistance)
                        .name(&self.data.name)
                        .width(time_slice),
                )
            })
            .collect();

        self.bars = bars;
    }

    fn chart(&self) -> BarChart {
        BarChart::new(&self.data.name, self.bars.clone())
            .element_formatter(Box::new(Self::format_element_percentage))
    }

    pub fn format_element_percentage(bar: &Bar, _: &BarChart) -> String {
        let mut formatter = NumberFormatter::new();
        if bar.name.is_empty() {
            return format!("{}%", formatter.format(bar.value, 2));
        }
        format!("{}\n{}%", bar.name, formatter.format(bar.value, 2))
    }
}
