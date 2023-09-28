use eframe::egui::*;
use egui_plot::*;

use super::common::*;

pub struct ValuesChart<T: PreparedValue> {
    newly_created: bool,
    bars: Vec<Bars<T>>,
    updated_time_slice: Option<f64>,
}

pub type DamageChart = ValuesChart<PreparedHitValue>;
pub type HealChart = ValuesChart<PreparedHealValue>;

struct Bars<T: PreparedValue> {
    data: PreparedDataSet<T>,
    bars: Vec<Bar>,
}

impl<T: PreparedValue> ValuesChart<T> {
    pub fn empty() -> Self {
        Self {
            newly_created: true,
            bars: Vec::new(),
            updated_time_slice: None,
        }
    }

    pub fn from_data(bars: impl Iterator<Item = PreparedDataSet<T>>, time_slice: f64) -> Self {
        let mut bars: Vec<_> = bars.map(|d| Bars::new(d)).collect();
        bars.sort_unstable_by(|b1, b2| {
            b1.data
                .total_value
                .total_cmp(&b2.data.total_value)
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

impl<T: PreparedValue> Bars<T> {
    fn new(data: PreparedDataSet<T>) -> Self {
        Self {
            data,
            bars: Vec::new(),
        }
    }

    fn update(&mut self, time_slice: f64) {
        let bars = time_slices(&self.data, time_slice)
            .filter_map(|(m, s)| {
                let value = s.iter().map(|p| p.value()).sum();
                if value == 0.0 {
                    return None;
                }

                Some(Bar::new(m, value).name(&self.data.name).width(time_slice))
            })
            .collect();

        self.bars = bars;
    }

    fn chart(&self) -> BarChart {
        BarChart::new(self.bars.clone())
            .element_formatter(Box::new(format_element))
            .name(&self.data.name)
    }
}
