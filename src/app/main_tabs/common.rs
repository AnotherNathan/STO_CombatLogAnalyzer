use chrono::Duration;
use eframe::egui::*;

use crate::{
    analyzer::{ShieldHullOptionalValues, ShieldHullValues},
    custom_widgets::table::*,
    helpers::{format_duration, number_formatting::NumberFormatter},
};

pub const ROW_HEIGHT: f32 = 25.0;
pub const HEADER_HEIGHT: f32 = 15.0;

#[derive(Default)]
pub struct TextValue {
    pub text: Option<String>,
    pub value: Option<f64>,
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

    pub fn option(
        value: &ShieldHullOptionalValues,
        precision: usize,
        number_formatter: &mut NumberFormatter,
    ) -> Self {
        Self {
            all: TextValue::option(value.all, precision, number_formatter),
            shield: value
                .shield
                .map(|s| number_formatter.format(s, precision))
                .unwrap_or_default(),
            hull: value
                .hull
                .map(|h| number_formatter.format(h, precision))
                .unwrap_or_default(),
        }
    }

    pub fn show(&self, row: &mut TableRow) {
        let response = self.all.show(row);
        if let Some(response) = response {
            show_shield_hull_values_tool_tip(response, &self.shield, &self.hull);
        }
    }
}

impl TextValue {
    pub fn new(value: f64, precision: usize, number_formatter: &mut NumberFormatter) -> Self {
        Self {
            text: Some(number_formatter.format(value, precision)),
            value: Some(value),
        }
    }

    pub fn option(
        value: Option<f64>,
        precision: usize,
        number_formatter: &mut NumberFormatter,
    ) -> Self {
        if let Some(value) = value {
            return Self {
                text: Some(number_formatter.format(value, precision)),
                value: Some(value),
            };
        }

        return Self {
            text: None,
            value: None,
        };
    }

    pub fn show(&self, row: &mut TableRow) -> Option<Response> {
        if let Some(text) = &self.text {
            return Some(show_value_text(row, text));
        }

        row.column(|_| {});
        None
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
            text: format_duration(duration),
            duration,
        }
    }

    pub fn show(&self, row: &mut TableRow) -> Response {
        show_value_text(row, &self.text)
    }
}

fn show_value_text(row: &mut TableRow, value_text: &str) -> Response {
    row.column(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(value_text)
        });
    })
}

pub fn show_shield_hull_values_tool_tip(response: Response, shield_value: &str, hull_value: &str) {
    response.on_hover_ui(|ui| {
        Table::new(ui).body(ROW_HEIGHT, |t| {
            t.row(|r| {
                r.column(|ui| {
                    ui.label("Shield");
                });
                show_value_text(r, shield_value);
            });
            t.row(|r| {
                r.column(|ui| {
                    ui.label("Hull");
                });
                show_value_text(r, hull_value);
            });
        });
    });
}

impl Default for TextDuration {
    fn default() -> Self {
        Self {
            text: Default::default(),
            duration: Duration::zero(),
        }
    }
}
