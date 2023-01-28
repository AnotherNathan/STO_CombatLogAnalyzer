use std::ops::Range;

use chrono::{Duration, NaiveDateTime, NaiveTime};
use eframe::egui::*;
use egui_extras::*;

use crate::{analyzer::ShieldHullValues, helpers::number_formatting::NumberFormatter};

pub const ROW_HEIGHT: f32 = 20.0;

#[derive(Default)]
pub struct TextValue {
    pub text: String,
    pub value: f64,
}

#[derive(Default)]
pub struct TextCount {
    pub text: String,
    pub count: u64,
}

#[derive(Default)]
pub struct ShieldAndHullTextValue {
    pub all: TextValue,
    pub shield: String,
    pub hull: String,
}

pub struct TextDuration {
    pub text: String,
    pub duration: Duration,
}

impl ShieldAndHullTextValue {
    pub fn new(
        value: &ShieldHullValues,
        precision: usize,
        number_formatter: &mut NumberFormatter,
    ) -> Self {
        Self {
            all: TextValue::new(value.all, precision, number_formatter),
            shield: number_formatter.format(value.shield, precision),
            hull: number_formatter.format(value.hull, precision),
        }
    }

    pub fn show(&self, row: &mut TableRow) {
        let response = self.all.show(row);
        show_shield_hull_values_tool_tip(response, &self.shield, &self.hull);
    }
}

impl TextValue {
    pub fn new(value: f64, precision: usize, number_formatter: &mut NumberFormatter) -> Self {
        Self {
            text: number_formatter.format(value, precision),
            value,
        }
    }

    pub fn show(&self, row: &mut TableRow) -> Response {
        show_value_text(row, &self.text)
    }
}

impl TextCount {
    pub fn new(count: u64) -> Self {
        Self {
            text: count.to_string(),
            count,
        }
    }

    pub fn show(&self, row: &mut TableRow) -> Response {
        show_value_text(row, &self.text)
    }
}

impl TextDuration {
    pub fn new(duration: Duration) -> Self {
        Self {
            text: Self::format(duration),
            duration,
        }
    }

    fn format(duration: Duration) -> String {
        let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap() + duration;
        if duration >= Duration::hours(1) {
            return format!("{}", time.format("%T%.3f"));
        }
        format!("{}", time.format("%M:%S%.3f"))
    }

    pub fn show(&self, row: &mut TableRow) -> Response {
        show_value_text(row, &self.text)
    }
}

fn show_value_text(row: &mut TableRow, value_text: &str) -> Response {
    row.col(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(value_text)
        });
    })
    .1
}

pub fn show_shield_hull_values_tool_tip(response: Response, shield_value: &str, hull_value: &str) {
    response.on_hover_ui(|ui| {
        TableBuilder::new(ui)
            .columns(Column::auto().at_least(60.0), 1)
            .columns(Column::auto(), 1)
            .body(|mut t| {
                t.row(ROW_HEIGHT, |mut r| {
                    r.col(|ui| {
                        ui.label("Shield");
                    });
                    show_value_text(&mut r, shield_value);
                });
                t.row(ROW_HEIGHT, |mut r| {
                    r.col(|ui| {
                        ui.label("Hull");
                    });
                    show_value_text(&mut r, hull_value);
                });
            });
    });
}

pub fn time_range_to_duration(time_range: &Range<NaiveDateTime>) -> Duration {
    time_range.end.signed_duration_since(time_range.start)
}

pub fn time_range_to_duration_or_zero(time_range: &Option<Range<NaiveDateTime>>) -> Duration {
    time_range
        .as_ref()
        .map(time_range_to_duration)
        .unwrap_or(Duration::zero())
}

impl Default for TextDuration {
    fn default() -> Self {
        Self {
            text: Default::default(),
            duration: Duration::zero(),
        }
    }
}
