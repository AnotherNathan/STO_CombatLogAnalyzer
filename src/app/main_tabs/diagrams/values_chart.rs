use eframe::egui::*;
use egui_plot::*;
use itertools::Itertools;

use crate::helpers::number_formatting::NumberFormatter;

use super::common::*;

pub struct ValuesChart<T: PreparedValue> {
    value_name: &'static str,
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
    pub fn empty(value_name: &'static str) -> Self {
        Self {
            value_name,
            newly_created: true,
            bars: Vec::new(),
            updated_time_slice: None,
        }
    }

    pub fn from_data(
        value_name: &'static str,
        bars: impl Iterator<Item = PreparedDataSet<T>>,
        time_slice: f64,
    ) -> Self {
        let bars: Vec<_> = bars.map(|d| Bars::new(d)).collect();
        let mut _self = Self {
            value_name,
            newly_created: true,
            bars,
            updated_time_slice: Some(time_slice),
        };
        _self.sort();
        _self
    }

    pub fn add_bars(&mut self, bars: PreparedDataSet<T>, time_slice: f64) {
        self.bars.push(Bars::new(bars));
        self.sort();
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

        let mut plot = Plot::new("value chart")
            .auto_bounds(true)
            .y_axis_formatter(format_axis)
            .x_axis_formatter(format_axis)
            .label_formatter(|_, p| {
                let mut formatter = NumberFormatter::new();
                format!(
                    "{}: {}\nTime: {}",
                    self.value_name,
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

    fn sort(&mut self) {
        self.bars.sort_unstable_by(|b1, b2| {
            b1.data
                .total_value
                .total_cmp(&b2.data.total_value)
                .reverse()
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
        BarChart::new(&self.data.name, self.bars.clone())
            .element_formatter(Box::new(format_element))
    }
}
