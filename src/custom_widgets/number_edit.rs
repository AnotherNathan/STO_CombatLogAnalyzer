use std::{hash::Hash, str::FromStr};

use eframe::egui::*;

pub struct NumberEdit<'a, T: FromStr + Ord + ToString + Copy> {
    value: &'a mut T,
    id: Id,
    desired_text_edit_width: Option<f32>,
    clamp_min: Option<T>,
    clamp_max: Option<T>,
}

#[derive(Clone, Default)]
struct State {
    value_text: String,
    is_editing_value_text: bool,
}

impl<'a, T: FromStr + Ord + ToString + Copy> NumberEdit<'a, T> {
    pub fn new(value: &'a mut T, id_source: impl Hash) -> Self {
        Self {
            value,
            id: Id::new(id_source),
            desired_text_edit_width: None,
            clamp_min: None,
            clamp_max: None,
        }
    }

    pub fn desired_text_edit_width(mut self, desired_with: f32) -> Self {
        self.desired_text_edit_width = Some(desired_with);
        self
    }

    pub fn clamp_min(mut self, min: T) -> Self {
        self.clamp_min = Some(min);
        self
    }

    pub fn clamp_max(mut self, max: T) -> Self {
        self.clamp_max = Some(max);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let Self {
            value,
            desired_text_edit_width,
            id,
            clamp_min,
            clamp_max,
        } = self;
        let mut state = State::load(ui.ctx(), id, value);

        let mut text_edit = TextEdit::singleline(&mut state.value_text);
        if let Some(desired_text_edit_width) = desired_text_edit_width {
            text_edit = text_edit.desired_width(desired_text_edit_width);
        }

        let text_edit_response = text_edit.show(ui).response;
        if text_edit_response.changed() {
            if let Ok(new_value) = state.value_text.parse::<T>() {
                *value = new_value;
            }
        }

        if let Some(clamp_min) = clamp_min {
            *value = (*value).max(clamp_min);
        }

        if let Some(clamp_max) = clamp_max {
            *value = (*value).min(clamp_max);
        }

        if text_edit_response.lost_focus() {
            state.value_text = value.to_string();
        }

        state.is_editing_value_text = text_edit_response.has_focus();

        state.store(ui.ctx(), id);

        text_edit_response
    }
}

impl State {
    fn load(ctx: &Context, id: Id, value: &impl ToString) -> Self {
        ctx.data_mut(|data| {
            let state = data.get_temp_mut_or_default::<Self>(id);

            if state.is_editing_value_text {
                return state.clone();
            }

            Self {
                value_text: value.to_string(),
                is_editing_value_text: false,
            }
        })
    }

    fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_temp(id, self));
    }
}
