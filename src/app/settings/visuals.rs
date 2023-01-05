use eframe::{
    egui::{ComboBox, Context, Ui, Visuals},
    Frame,
};

use crate::custom_widgets::slider_text_edit::SliderTextEdit;

use super::{app_settings::Theme, Settings};

#[derive(Default)]
pub struct VisualsTab {}

impl VisualsTab {
    pub fn show(&mut self, modified_settings: &mut Settings, ui: &mut Ui, frame: &Frame) {
        let visuals = &mut modified_settings.visuals;
        ui.label("Theme");
        ComboBox::from_id_source("theme combo box")
            .selected_text(visuals.theme.display())
            .show_ui(ui, |ui| {
                if ui
                    .selectable_value(&mut visuals.theme, Theme::Dark, Theme::Dark.display())
                    .changed()
                {
                    Self::set_theme(ui.ctx(), visuals.theme);
                }
                if ui
                    .selectable_value(&mut visuals.theme, Theme::Light, Theme::Light.display())
                    .changed()
                {
                    Self::set_theme(ui.ctx(), visuals.theme);
                }
            });

        ui.add_space(10.0);
        ui.separator();

        ui.label("UI Scale");
        let response = SliderTextEdit::new(&mut visuals.ui_scale, 0.5..=3.0, "ui scale slider")
            .clamp_to_range(false)
            .clamp_min(0.5)
            .clamp_max(10.0)
            .step_by(0.1)
            .display_precision(4)
            .desired_text_edit_width(40.0)
            .show(ui);
        if response.drag_released() || response.lost_focus() {
            Self::set_ui_scale(
                ui.ctx(),
                frame.info().native_pixels_per_point,
                visuals.ui_scale,
            );
        }
    }

    pub fn update_visuals(
        &mut self,
        ctx: &Context,
        native_pixels_per_point: Option<f32>,
        settings: &Settings,
    ) {
        let visuals = &settings.visuals;
        Self::set_theme(ctx, visuals.theme);
        Self::set_ui_scale(ctx, native_pixels_per_point, visuals.ui_scale);
    }

    fn set_theme(ctx: &Context, theme: Theme) {
        let visuals = match theme {
            Theme::Dark => Visuals::dark(),
            Theme::Light => Visuals::light(),
        };
        ctx.set_visuals(visuals);
    }

    fn set_ui_scale(ctx: &Context, native_pixels_per_point: Option<f32>, ui_scale: f64) {
        ctx.set_pixels_per_point(native_pixels_per_point.unwrap_or(1.0) * ui_scale as f32);
    }
}
