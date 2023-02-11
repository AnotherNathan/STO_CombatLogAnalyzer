use std::hash::Hash;
use std::ops::RangeInclusive;

use eframe::egui::{Context, Id, Response, Slider, TextEdit, Ui, Widget};

pub struct SliderTextEdit<'a> {
    value: &'a mut f64,
    range: RangeInclusive<f64>,
    id: Id,
    desired_text_edit_width: Option<f32>,
    step: Option<f64>,
    clamp_to_range: Option<bool>,
    clamp_min: Option<f64>,
    clamp_max: Option<f64>,
    display_precision: Option<i32>,
}

#[derive(Clone, Default)]
struct State {
    value_text: String,
    is_editing_value_text: bool,
}

impl<'a> SliderTextEdit<'a> {
    pub fn new(value: &'a mut f64, range: RangeInclusive<f64>, id_source: impl Hash) -> Self {
        Self {
            value,
            range,
            id: Id::new(id_source),
            desired_text_edit_width: None,
            step: None,
            clamp_to_range: None,
            clamp_min: None,
            clamp_max: None,
            display_precision: None,
        }
    }

    pub fn clamp_to_range(mut self, clamp_to_range: bool) -> Self {
        self.clamp_to_range = Some(clamp_to_range);
        self
    }

    pub fn step_by(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    pub fn desired_text_edit_width(mut self, desired_with: f32) -> Self {
        self.desired_text_edit_width = Some(desired_with);
        self
    }

    pub fn clamp_min(mut self, min: f64) -> Self {
        self.clamp_min = Some(min);
        self
    }

    pub fn clamp_max(mut self, max: f64) -> Self {
        self.clamp_max = Some(max);
        self
    }

    pub fn display_precision(mut self, display_precision: i32) -> Self {
        self.display_precision = Some(display_precision);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let Self {
            value,
            range,
            desired_text_edit_width,
            step,
            clamp_to_range,
            id,
            clamp_min,
            clamp_max,
            display_precision,
        } = self;
        ui.horizontal(|ui| {
            let mut state = State::load(ui.ctx(), id, value, display_precision);
            let mut slider = Slider::new(value, range).show_value(false);
            if let Some(clamp_to_range) = clamp_to_range {
                slider = slider.clamp_to_range(clamp_to_range);
            }

            if let Some(step) = step {
                slider = slider.step_by(step);
            }

            let slider_response = slider.ui(ui);

            let mut text_edit = TextEdit::singleline(&mut state.value_text);
            if let Some(desired_text_edit_width) = desired_text_edit_width {
                text_edit = text_edit.desired_width(desired_text_edit_width);
            }

            let text_edit_response = text_edit.show(ui).response;
            if text_edit_response.changed() {
                if let Ok(new_value) = state.value_text.parse::<f64>() {
                    *value = new_value;
                }
            }

            if let Some(clamp_min) = clamp_min {
                *value = value.max(clamp_min);
            }

            if let Some(clamp_max) = clamp_max {
                *value = value.min(clamp_max);
            }

            if slider_response.changed() || text_edit_response.lost_focus() {
                state.value_text = Self::format_value(*value, display_precision);
            }

            state.is_editing_value_text = text_edit_response.has_focus();

            state.store(ui.ctx(), id);

            text_edit_response.union(slider_response)
        })
        .inner
    }

    fn format_value(mut value: f64, display_precision: Option<i32>) -> String {
        if let Some(display_precision) = display_precision {
            let multiplier = 10.0f64.powi(display_precision);
            value = value * multiplier;
            value = value.round();
            value = value / multiplier;
        }
        format!("{}", value)
    }
}

impl State {
    fn load(ctx: &Context, id: Id, value: &f64, display_precision: Option<i32>) -> Self {
        ctx.data_mut(|data| {
            let state = data.get_temp_mut_or_default::<Self>(id);

            if state.is_editing_value_text {
                return state.clone();
            }

            Self {
                value_text: SliderTextEdit::format_value(*value, display_precision),
                is_editing_value_text: false,
            }
        })
    }

    fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_temp(id, self));
    }
}
