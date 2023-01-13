use std::{
    cmp::Ordering,
    f64::consts::PI,
    ops::{Range, RangeInclusive},
};

use eframe::egui::{widgets::plot::*, *};

use crate::{
    analyzer::{DamageGroup, Hit},
    helpers::number_formatting::NumberFormatter,
};

const SAMPLE_RATE: f64 = 10.0;

pub struct DpsPlot {
    lines: Vec<PreparedLine>,
    largest_point: f64,
    newly_created: bool,
    updated_filter: Option<Filter>,
}

pub struct PreparedLine {
    name: String,
    points: Vec<[f64; 2]>,
    summed_and_sorted_hits: Vec<Hit>,
    start_time_s: f64,
    duration_s: f64,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Filter {
    Gauss { standard_deviation: f64 },
    Triangle { size: f64 },
    Box { size: f64 },
}

impl DpsPlot {
    pub fn empty() -> Self {
        Self {
            lines: Vec::new(),
            largest_point: 100_000.0,
            newly_created: true,
            updated_filter: None,
        }
    }

    pub fn from_damage_groups<'a>(
        groups: impl Iterator<Item = &'a DamageGroup>,
        filter: Filter,
    ) -> Self {
        Self::from_data(
            groups.map(|g| {
                (
                    g.name.as_str(),
                    g.hull_hits.iter().chain(g.shield_hits.iter()),
                )
            }),
            filter,
        )
    }

    pub fn from_data<'a>(
        lines: impl Iterator<Item = (&'a str, impl Iterator<Item = &'a Hit>)>,
        filter: Filter,
    ) -> Self {
        let lines: Vec<_> = lines
            .map(|(n, h)| PreparedLine::new(n, h, filter))
            .collect();
        let largest_point = Self::compute_largest_point(&lines);
        Self {
            lines,
            largest_point,
            newly_created: true,
            updated_filter: None,
        }
    }

    pub fn update(&mut self, filter: Filter) {
        self.updated_filter = Some(filter);
    }

    pub fn show(&mut self, ui: &mut Ui) {
        if let Some(filter) = self.updated_filter.take() {
            self.lines.iter_mut().for_each(|l| l.update(filter));
            self.largest_point = Self::compute_largest_point(&self.lines);
        }

        let mut plot = Plot::new("dps plot")
            .y_axis_formatter(Self::format_axis)
            .x_axis_formatter(Self::format_axis)
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

    fn format_axis(value: f64, _: &RangeInclusive<f64>) -> String {
        if value < 0.0 {
            return String::new();
        }
        let mut formatter = NumberFormatter::new();
        formatter.format(value, 0)
    }

    fn format_label(name: &str, point: &PlotPoint) -> String {
        if point.x < 0.0 || point.y < 0.0 {
            return String::new();
        }

        let mut formatter = NumberFormatter::new();
        let x = formatter.format(point.x, 2);
        let y = formatter.format(point.y, 2);
        format!("{}\nDPS: {}\nTime: {}", name, y, x)
    }

    fn compute_largest_point(lines: &[PreparedLine]) -> f64 {
        lines
            .iter()
            .flat_map(|l| l.points.iter())
            .map(|p| p[1])
            .max_by(|p1, p2| p1.total_cmp(p2))
            .unwrap_or(0.0)
    }
}

impl PreparedLine {
    fn new<'a>(name: &str, hits: impl Iterator<Item = &'a Hit>, filter_method: Filter) -> Self {
        let summed_and_sorted_hits = Self::sum_up_and_sort_hits(hits);

        let start_time_s = summed_and_sorted_hits
            .iter()
            .map(|h| h.times_millis)
            .min()
            .unwrap_or(0) as f64
            / 1e3;
        let end_time_s = summed_and_sorted_hits
            .iter()
            .map(|h| h.times_millis)
            .max()
            .unwrap_or(0) as f64
            / 1e3;

        let duration_s = end_time_s - start_time_s;

        let mut prepared_line = Self {
            name: name.to_string(),
            points: Vec::new(),
            start_time_s,
            duration_s,
            summed_and_sorted_hits,
        };

        prepared_line.update(filter_method);

        prepared_line
    }

    fn sum_up_and_sort_hits<'a>(hits: impl Iterator<Item = &'a Hit>) -> Vec<Hit> {
        let mut hits = Vec::from_iter(hits.copied());
        hits.sort_unstable_by_key(|h| h.times_millis);
        hits.dedup_by(|h1, h2| {
            if h1.times_millis != h2.times_millis {
                return false;
            }

            h2.damage += h1.damage;
            true
        });
        hits.shrink_to_fit();
        hits
    }

    fn update(&mut self, filter: Filter) {
        let (sample, filter_value_s): (fn(&[Hit], f64, f64) -> f64, f64) = match filter {
            Filter::Gauss { standard_deviation } => (Self::sample_gauss_filter, standard_deviation),
            Filter::Triangle { size } => (Self::sample_triangle_filter, size),
            Filter::Box { size } => (Self::sample_box_filter, size),
        };

        let points_count = (self.duration_s * SAMPLE_RATE).round() as _;
        let mut points = Vec::with_capacity(points_count);
        for i in 0..points_count {
            let start_offset = i as f64 / (points_count - 1) as f64;
            let time = self.start_time_s + self.duration_s * start_offset;
            let point = [
                time,
                sample(&self.summed_and_sorted_hits, time, filter_value_s),
            ];
            points.push(point);
        }

        self.points = points;
    }

    fn get_sample_entry(hits: &[Hit], time_range_millis: Range<u32>) -> Option<usize> {
        hits.binary_search_by(|h| {
            if h.times_millis < time_range_millis.start {
                return Ordering::Less;
            }

            if h.times_millis >= time_range_millis.end {
                return Ordering::Greater;
            }

            Ordering::Equal
        })
        .ok()
    }

    fn get_sample_entry2(hits: &[Hit], time_millis: u32) -> usize {
        match hits.binary_search_by_key(&time_millis, |h| h.times_millis) {
            Ok(i) => i,
            Err(i) => i,
        }
    }

    fn get_sample_hits<'a>(
        hits: &'a [Hit],
        filter_size_seconds: f64,
        time_seconds: f64,
    ) -> &'a [Hit] {
        let filter_size_millis = seconds_to_millis(filter_size_seconds);
        let filter_half_size = filter_size_millis / 2;
        let time_millis = seconds_to_millis(time_seconds);
        let time_start_millis = time_millis.saturating_sub(filter_half_size);
        let time_end_millis = time_millis + filter_half_size;

        let index = match Self::get_sample_entry(hits, time_start_millis..time_end_millis) {
            Some(i) => i,
            None => return &[],
        };

        let start_index = index
            - hits[0..index]
                .iter()
                .rev()
                .take_while(|h| h.times_millis >= time_start_millis)
                .count();
        let end_index = index
            + hits[index..]
                .iter()
                .take_while(|h| h.times_millis < time_end_millis)
                .count();
        &hits[start_index..end_index]
    }

    fn sample_box_filter(hits: &[Hit], time_seconds: f64, filter_size_seconds: f64) -> f64 {
        let hits = Self::get_sample_hits(hits, filter_size_seconds, time_seconds);

        let value = hits.iter().map(|h| h.damage).sum::<f64>() / filter_size_seconds;

        value
    }

    fn sample_triangle_filter(hits: &[Hit], time_seconds: f64, filter_size_seconds: f64) -> f64 {
        let hits = Self::get_sample_hits(hits, filter_size_seconds, time_seconds);
        let half_size = filter_size_seconds * 0.5;

        let mut value = 0.0;
        for hit in hits {
            let offset_to_triangle_center =
                (time_seconds - millis_to_seconds(hit.times_millis)).abs();
            let weight = (half_size - offset_to_triangle_center) / (half_size * half_size);
            value += weight * hit.damage;
        }

        value
    }

    fn gauss_probability_density_function(t: f64, offset: f64, standard_deviation: f64) -> f64 {
        let t_sub_off_over_sigma = (t - offset) / standard_deviation;
        1.0 / (standard_deviation * f64::sqrt(2.0 * PI))
            * f64::exp(-0.5 * t_sub_off_over_sigma * t_sub_off_over_sigma)
    }

    fn get_gauss_value(
        hits: &[Hit],
        index: usize,
        time_seconds: f64,
        sigma_seconds: f64,
    ) -> Option<f64> {
        let hit = hits.get(index)?;
        let t = millis_to_seconds(hit.times_millis);
        let finite_hack_value = 1e-3;
        let weight = (Self::gauss_probability_density_function(t, time_seconds, sigma_seconds)
            - finite_hack_value)
            * (1.0 + finite_hack_value);
        if weight <= 0.0 {
            return None;
        }

        Some(weight * hit.damage)
    }

    fn get_gauss_sample_half(
        hits: &[Hit],
        time_seconds: f64,
        sigma_seconds: f64,
        entry_index: usize,
        mut index_change: impl FnMut(usize) -> Option<usize>,
    ) -> f64 {
        let mut value = 0.0;
        let mut index = entry_index;
        loop {
            value += match Self::get_gauss_value(hits, index, time_seconds, sigma_seconds) {
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

    fn sample_gauss_filter(hits: &[Hit], time_seconds: f64, sigma_seconds: f64) -> f64 {
        let time_millis = seconds_to_millis(time_seconds);

        let entry_index = Self::get_sample_entry2(hits, time_millis);

        Self::get_gauss_sample_half(hits, time_seconds, sigma_seconds, entry_index - 1, |i| {
            i.checked_sub(1)
        }) + Self::get_gauss_value(hits, entry_index, time_seconds, sigma_seconds).unwrap_or(0.0)
            + Self::get_gauss_sample_half(hits, time_seconds, sigma_seconds, entry_index + 1, |i| {
                Some(i + 1)
            })
    }

    fn to_line(&self) -> Line {
        Line::new(self.points.clone()).name(&self.name).width(2.0)
    }
}

fn seconds_to_millis(seconds: f64) -> u32 {
    (seconds * 1e3).round() as _
}

fn millis_to_seconds(millis: u32) -> f64 {
    millis as f64 * (1.0 / 1e3)
}

impl Filter {
    pub const fn display_name(&self) -> &'static str {
        match self {
            Filter::Gauss { .. } => "Gauss",
            Filter::Triangle { .. } => "Triangle",
            Filter::Box { .. } => "Box",
        }
    }

    pub const fn display_value_name(&self) -> &'static str {
        match self {
            Filter::Gauss { .. } => "Standard Deviation (s)",
            Filter::Triangle { .. } | Filter::Box { .. } => "Size (s)",
        }
    }

    pub const fn recommended_value_range(&self) -> RangeInclusive<f64> {
        match self {
            Filter::Gauss { .. } => 0.4..=6.0,
            Filter::Triangle { .. } => 2.0..=30.0,
            Filter::Box { .. } => 1.0..=30.0,
        }
    }

    pub fn value_mut(&mut self) -> &mut f64 {
        match self {
            Filter::Gauss { standard_deviation } => standard_deviation,
            Filter::Triangle { size } => size,
            Filter::Box { size } => size,
        }
    }
}
