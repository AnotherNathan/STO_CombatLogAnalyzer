use std::{cmp::Ordering, ops::RangeInclusive};

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
    updated_filter_parameters: Option<(f64, FilterMethod)>,
}

pub struct PreparedLine {
    name: String,
    points: Vec<[f64; 2]>,
    summed_and_sorted_hits: Vec<Hit>,
    start_time_s: f64,
    duration_s: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FilterMethod {
    Triangle,
    Box,
}

impl DpsPlot {
    pub fn empty() -> Self {
        Self {
            lines: Vec::new(),
            largest_point: 100_000.0,
            newly_created: true,
            updated_filter_parameters: None,
        }
    }

    pub fn from_damage_groups<'a>(
        groups: impl Iterator<Item = &'a DamageGroup>,
        filter_size_s: f64,
        filter_method: FilterMethod,
    ) -> Self {
        Self::from_data(
            groups.map(|g| {
                (
                    g.name.as_str(),
                    g.hull_hits.iter().chain(g.shield_hits.iter()),
                )
            }),
            filter_size_s,
            filter_method,
        )
    }

    pub fn from_data<'a>(
        lines: impl Iterator<Item = (&'a str, impl Iterator<Item = &'a Hit>)>,
        filter_size_s: f64,
        filter_method: FilterMethod,
    ) -> Self {
        let lines: Vec<_> = lines
            .map(|(n, h)| PreparedLine::new(n, h, filter_size_s, filter_method))
            .collect();
        let largest_point = Self::compute_largest_point(&lines);
        Self {
            lines,
            largest_point,
            newly_created: true,
            updated_filter_parameters: None,
        }
    }

    pub fn update(&mut self, filter_size_s: f64, filter_method: FilterMethod) {
        self.updated_filter_parameters = Some((filter_size_s, filter_method));
    }

    pub fn show(&mut self, ui: &mut Ui) {
        if let Some((filter_size_s, filter_method)) = self.updated_filter_parameters.take() {
            self.lines
                .iter_mut()
                .for_each(|l| l.update(filter_size_s, filter_method));
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
    fn new<'a>(
        name: &str,
        hits: impl Iterator<Item = &'a Hit>,
        filter_size_s: f64,
        filter_method: FilterMethod,
    ) -> Self {
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

        prepared_line.update(filter_size_s, filter_method);

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

    fn update(&mut self, filter_size_s: f64, filter_method: FilterMethod) {
        let filter_size_m = seconds_to_millis(filter_size_s);
        let sample = match filter_method {
            FilterMethod::Triangle => Self::sample_triangle_filter,
            FilterMethod::Box => Self::sample_box_filter,
        };

        let points_count = (self.duration_s * SAMPLE_RATE).round() as _;
        let mut points = Vec::with_capacity(points_count);
        for i in 0..points_count {
            let start_offset = i as f64 / (points_count - 1) as f64;
            let time = self.start_time_s + self.duration_s * start_offset;
            let point = [
                time,
                sample(
                    &self.summed_and_sorted_hits,
                    time,
                    filter_size_m,
                    filter_size_s,
                ),
            ];
            points.push(point);
        }

        self.points = points;
    }

    fn get_sample_hits<'a>(
        hits: &'a [Hit],
        filter_size_millis: u32,
        time_seconds: f64,
    ) -> &'a [Hit] {
        let filter_half_size = filter_size_millis / 2;
        let time_millis = seconds_to_millis(time_seconds);
        let time_range_millis =
            (time_millis.saturating_sub(filter_half_size))..(time_millis + filter_half_size);

        let index = match hits.binary_search_by(|h| {
            if h.times_millis < time_range_millis.start {
                return Ordering::Less;
            }

            if h.times_millis >= time_range_millis.end {
                return Ordering::Greater;
            }

            Ordering::Equal
        }) {
            Ok(i) => i,
            Err(_) => return &[],
        };

        let start_index = index
            - hits[0..index]
                .iter()
                .rev()
                .take_while(|h| h.times_millis >= time_range_millis.start)
                .count();
        let end_index = index
            + hits[index..]
                .iter()
                .take_while(|h| h.times_millis < time_range_millis.end)
                .count();
        &hits[start_index..end_index]
    }

    fn sample_box_filter(
        hits: &[Hit],
        time_seconds: f64,
        filter_size_millis: u32,
        filter_size_seconds: f64,
    ) -> f64 {
        let hits = Self::get_sample_hits(hits, filter_size_millis, time_seconds);

        let value = hits.iter().map(|h| h.damage).sum::<f64>() / filter_size_seconds;

        value
    }

    fn sample_triangle_filter(
        hits: &[Hit],
        time_seconds: f64,
        filter_size_millis: u32,
        filter_size_seconds: f64,
    ) -> f64 {
        let hits = Self::get_sample_hits(hits, filter_size_millis, time_seconds);
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

impl FilterMethod {
    pub const fn display(&self) -> &'static str {
        match self {
            FilterMethod::Triangle => "Triangle",
            FilterMethod::Box => "Box",
        }
    }
}
