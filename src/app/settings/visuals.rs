use eframe::{
    egui::{style::Selection, ComboBox, Context, Ui, Visuals},
    epaint::{Color32, Rgba, Shadow},
};

use crate::custom_widgets::slider_text_edit::SliderTextEdit;

use super::{app_settings::Theme, Settings};

#[derive(Default)]
pub struct VisualsTab {}

impl VisualsTab {
    pub fn show(&mut self, modified_settings: &mut Settings, ui: &mut Ui) {
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
                    .selectable_value(
                        &mut visuals.theme,
                        Theme::LightDark,
                        Theme::LightDark.display(),
                    )
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
                ui.ctx().native_pixels_per_point(),
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
        let mut visuals = match theme {
            Theme::Dark => Visuals::dark(),
            Theme::LightDark => Self::light_dark(),
            Theme::Light => Visuals::light(),
        };
        visuals.panel_fill = Color32::from_rgba_premultiplied(
            visuals.panel_fill.r(),
            visuals.panel_fill.g(),
            visuals.panel_fill.b(),
            50,
        );
        ctx.set_visuals(visuals);
    }

    fn set_ui_scale(ctx: &Context, native_pixels_per_point: Option<f32>, ui_scale: f64) {
        ctx.set_pixels_per_point(native_pixels_per_point.unwrap_or(1.0) * ui_scale as f32);
    }

    fn light_dark() -> Visuals {
        let background = Rgba::from_rgb(0.08, 0.08, 0.08).into();
        let darker_background = Rgba::from_rgb(0.05, 0.05, 0.05).into();
        let brighter_background = Rgba::from_rgb(0.15, 0.15, 0.15).into();
        let mut theme = Visuals::dark();
        theme.code_bg_color = background;
        theme.error_fg_color = Rgba::from_rgb(0.8, 0.3, 0.3).into();
        theme.extreme_bg_color = darker_background;
        theme.faint_bg_color = brighter_background;
        theme.hyperlink_color = Rgba::from_rgb(0.2, 0.2, 0.9).into();
        theme.panel_fill = background;
        theme.warn_fg_color = Rgba::from_rgb(0.8, 0.7, 0.3).into();
        theme.override_text_color = Some(Rgba::from_rgb(0.92, 0.92, 0.92).into());
        theme.selection = Selection {
            bg_fill: Rgba::from_rgb(0.2, 0.2, 0.7).into(),
            ..Default::default()
        };
        theme.popup_shadow = Shadow::big_light();

        theme.widgets.inactive.bg_fill = Rgba::from_rgb(0.2, 0.2, 0.2).into();
        theme.widgets.hovered.bg_fill = Rgba::from_rgb(0.25, 0.25, 0.25).into();
        theme.widgets.active.bg_fill = Rgba::from_rgb(0.3, 0.3, 0.3).into();

        theme.window_fill = background;
        theme.window_stroke.color = Rgba::from_rgb(0.9, 0.9, 0.9).into();
        theme.window_shadow = Shadow::big_light();
        theme
    }
}
