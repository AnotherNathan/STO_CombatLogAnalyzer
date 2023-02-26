use eframe::egui::{plot::*, *};

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

    pub fn update(&mut self, time_slice: f64) {
        self.updated_time_slice = Some(time_slice);
    }

    pub fn show(&mut self, ui: &mut Ui) {
        if let Some(time_slice) = self.updated_time_slice.take() {
            self.bars.iter_mut().for_each(|b| b.update(time_slice));
        }

        let mut plot = Plot::new("damage resistance chart")
            .y_axis_formatter(format_axis)
            .x_axis_formatter(format_axis)
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
}

impl DamageResistanceBars {
    fn new(data: PreparedDamageDataSet) -> Self {
        Self {
            data,
            bars: Vec::new(),
        }
    }

    fn update(&mut self, time_slice: f64) {
        let time_slice_m = seconds_to_millis(time_slice);
        let mut bars = Vec::new();
        let first_time_slice = seconds_to_millis(self.data.start_time_s) / time_slice_m;
        let mut time_slice_end = first_time_slice + time_slice_m;
        let mut index = 0;
        let mut damage = 0.0;
        let mut shield_damage = 0.0;
        let mut hull_damage = 0.0;
        let mut drain_damage = 0.0;
        let mut base_damage = 0.0;
        loop {
            let hit = match self.data.values.get(index) {
                Some(h) => h,
                None => {
                    break;
                }
            };

            if hit.time_millis < time_slice_end {
                index += 1;
                damage += hit.damage;
                shield_damage += hit.shield_damage;
                hull_damage += hit.hull_damage;
                drain_damage += hit.drain_damage;
                base_damage += hit.base_damage;
                continue;
            }

            let total_damage = &ShieldHullValues {
                all: damage,
                shield: shield_damage,
                hull: hull_damage,
            };
            let resistance = damage_resistance_percentage(&total_damage, base_damage, drain_damage);

            let time = millis_to_seconds(time_slice_end - time_slice_m / 2);

            if let Some(resistance) = resistance {
                bars.push(
                    Bar::new(time, resistance)
                        .name(&self.data.name)
                        .width(time_slice),
                );
            }

            damage = 0.0;
            shield_damage = 0.0;
            hull_damage = 0.0;
            drain_damage = 0.0;
            base_damage = 0.0;
            time_slice_end += time_slice_m;
        }

        self.bars = bars;
    }

    fn chart(&self) -> BarChart {
        BarChart::new(self.bars.clone())
            .element_formatter(Box::new(Self::format_element_percentage))
            .name(&self.data.name)
    }

    pub fn format_element_percentage(bar: &Bar, _: &BarChart) -> String {
        let mut formatter = NumberFormatter::new();
        format!("{}\n{}%", bar.name, formatter.format(bar.value, 2))
    }
}
