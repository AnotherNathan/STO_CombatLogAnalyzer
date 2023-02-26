use std::f64::consts::PI;

use eframe::egui::{widgets::plot::*, *};

use crate::helpers::number_formatting::NumberFormatter;

use super::common::*;

const SAMPLE_RATE: f64 = 10.0;

pub struct ValuePerSecondGraph<T: PreparedValue> {
    lines: Vec<GraphLine<T>>,
    largest_point: f64,
    newly_created: bool,
    updated_filter: Option<f64>,
}

pub type DpsGraph = ValuePerSecondGraph<PreparedHitValue>;
pub type HpsGraph = ValuePerSecondGraph<PreparedHealValue>;

pub struct GraphLine<T: PreparedValue> {
    points: Vec<[f64; 2]>,
    data: PreparedDataSet<T>,
}

impl<T: PreparedValue> ValuePerSecondGraph<T> {
    pub fn empty() -> Self {
        Self {
            lines: Vec::new(),
            largest_point: 100_000.0,
            newly_created: true,
            updated_filter: None,
        }
    }

    pub fn from_data<'a>(lines: impl Iterator<Item = PreparedDataSet<T>>, filter: f64) -> Self {
        let lines: Vec<_> = lines.map(|l| GraphLine::new(l)).collect();
        let largest_point = Self::compute_largest_point(&lines);
        Self {
            lines,
            largest_point,
            newly_created: true,
            updated_filter: Some(filter),
        }
    }

    pub fn update(&mut self, filter: f64) {
        self.updated_filter = Some(filter);
    }

    pub fn show(&mut self, ui: &mut Ui) {
        if let Some(filter) = self.updated_filter.take() {
            self.lines.iter_mut().for_each(|l| l.update(filter));
            self.largest_point = Self::compute_largest_point(&self.lines);
        }

        let mut plot = Plot::new("dps graph")
            .y_axis_formatter(format_axis)
            .x_axis_formatter(format_axis)
            .label_formatter(Self::format_label)
            .include_y(self.largest_point)
            .legend(Legend::default());

        if self.newly_created {
            plot = plot.reset();
            self.newly_created = false;
        }

        if self.lines.len() == 0 {
            plot = plot.include_x(60.0);
        }

        plot.show(ui, |p| {
            for line in self.lines.iter() {
                p.line(line.to_line());
            }
        });
    }

    pub fn format_label(name: &str, point: &PlotPoint) -> String {
        if point.x < 0.0 || point.y < 0.0 {
            return String::new();
        }

        let mut formatter = NumberFormatter::new();
        let x = formatter.format(point.x, 2);
        let y = formatter.format(point.y, 2);
        format!("{}\nDPS: {}\nTime: {}", name, y, x)
    }

    fn compute_largest_point(lines: &[GraphLine<T>]) -> f64 {
        lines
            .iter()
            .flat_map(|l| l.points.iter())
            .map(|p| p[1])
            .max_by(|p1, p2| p1.total_cmp(p2))
            .unwrap_or(0.0)
    }
}

impl<T: PreparedValue> GraphLine<T> {
    fn new<'a>(data: PreparedDataSet<T>) -> Self {
        Self {
            points: Vec::new(),
            data,
        }
    }

    fn update(&mut self, filter: f64) {
        let points_count = (self.data.duration_s * SAMPLE_RATE).round() as _;
        let mut points = Vec::with_capacity(points_count);
        for i in 0..points_count {
            let start_offset = i as f64 / (points_count - 1) as f64;
            let time = self.data.start_time_s + self.data.duration_s * start_offset;
            let point = [
                time,
                Self::get_sample_gauss_filtered(&self.data.values, time, filter),
            ];
            points.push(point);
        }

        self.points = points;
    }

    fn get_sample_entry(points: &[PreparedPoint<T>], time_millis: u32) -> usize {
        match points.binary_search_by_key(&time_millis, |h| h.time_millis) {
            Ok(i) => i,
            Err(i) => i,
        }
    }

    fn gauss_probability_density_function(t: f64, offset: f64, standard_deviation: f64) -> f64 {
        let t_sub_off_over_sigma = (t - offset) / standard_deviation;
        1.0 / (standard_deviation * f64::sqrt(2.0 * PI))
            * f64::exp(-0.5 * t_sub_off_over_sigma * t_sub_off_over_sigma)
    }

    fn get_gauss_value(
        points: &[PreparedPoint<T>],
        index: usize,
        time_seconds: f64,
        sigma_seconds: f64,
    ) -> Option<f64> {
        let hit = points.get(index)?;
        let t = millis_to_seconds(hit.time_millis);
        let finite_hack_value = 1e-3;
        let weight = (Self::gauss_probability_density_function(t, time_seconds, sigma_seconds)
            - finite_hack_value)
            * (1.0 + finite_hack_value);
        if weight <= 0.0 {
            return None;
        }

        Some(weight * hit.value())
    }

    fn get_get_sample_gauss_filtered_half(
        points: &[PreparedPoint<T>],
        time_seconds: f64,
        sigma_seconds: f64,
        entry_index: usize,
        mut index_change: impl FnMut(usize) -> Option<usize>,
    ) -> f64 {
        let mut value = 0.0;
        let mut index = entry_index;
        loop {
            value += match Self::get_gauss_value(points, index, time_seconds, sigma_seconds) {
                Some(v) => v,
                None => break,
            };
            index = match index_change(index) {
                Some(i) => i,
                None => break,
            };
        }

        value
    }

    fn get_sample_gauss_filtered(
        points: &[PreparedPoint<T>],
        time_seconds: f64,
        sigma_seconds: f64,
    ) -> f64 {
        let time_millis = seconds_to_millis(time_seconds);

        let entry_index = Self::get_sample_entry(points, time_millis);

        entry_index
            .checked_sub(1)
            .map(|i| {
                Self::get_get_sample_gauss_filtered_half(
                    points,
                    time_seconds,
                    sigma_seconds,
                    i,
                    |i| i.checked_sub(1),
                )
            })
            .unwrap_or(0.0)
            + Self::get_gauss_value(points, entry_index, time_seconds, sigma_seconds).unwrap_or(0.0)
            + Self::get_get_sample_gauss_filtered_half(
                points,
                time_seconds,
                sigma_seconds,
                entry_index + 1,
                |i| Some(i + 1),
            )
    }

    fn to_line(&self) -> Line {
        Line::new(self.points.clone())
            .name(&self.data.name)
            .width(2.0)
    }
}
