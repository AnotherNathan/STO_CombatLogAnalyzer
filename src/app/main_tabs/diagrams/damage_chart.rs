use eframe::egui::{plot::*, *};

use super::common::*;

pub struct DamageChart {
    newly_created: bool,
    bars: Vec<DamageBars>,
    updated_time_slice: Option<f64>,
}

struct DamageBars {
    data: PreparedDamageDataSet,
    bars: Vec<Bar>,
}

impl DamageChart {
    pub fn empty() -> Self {
        Self {
            newly_created: true,
            bars: Vec::new(),
            updated_time_slice: None,
        }
    }

    pub fn from_data(bars: impl Iterator<Item = PreparedDamageDataSet>, time_slice: f64) -> Self {
        let mut bars: Vec<_> = bars.map(|d| DamageBars::new(d)).collect();
        bars.sort_unstable_by(|b1, b2| {
            b1.data
                .total_damage
                .total_cmp(&b2.data.total_damage)
                .reverse()
        });
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

        let mut plot = Plot::new("damage chart")
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

impl DamageBars {
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
        loop {
            let hit = match self.data.hits.get(index) {
                Some(h) => h,
                None => {
                    break;
                }
            };

            if hit.time_millis < time_slice_end {
                index += 1;
                damage += hit.damage;
                continue;
            }

            if damage > 0.0 {
                let bar = Bar::new(millis_to_seconds(time_slice_end - time_slice_m / 2), damage)
                    .name(&self.data.name)
                    .width(time_slice);
                bars.push(bar);
            }
            damage = 0.0;
            time_slice_end += time_slice_m;
        }

        self.bars = bars;
    }

    fn chart(&self) -> BarChart {
        BarChart::new(self.bars.clone())
            .element_formatter(Box::new(format_element))
            .name(&self.data.name)
    }
}
