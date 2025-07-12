use std::ops::RangeInclusive;

use eframe::{egui::*, emath::GuiRounding};

#[must_use = "You should call .show()"]
pub struct Splitter {
    orientation: SplitterOrientation,
    initial_ratio: f32,
    spacing: f32,
    ratio_bounds: RangeInclusive<f32>,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum SplitterOrientation {
    Horizontal,
    Vertical,
}

#[allow(dead_code)]
pub struct SplitterResponse<R> {
    pub top_left_response: Response,
    pub bottom_right_response: Response,
    pub splitter_response: Response,
    pub inner: InnerResponse<R>,
    pub rect: Rect,
}

impl Splitter {
    pub fn with_orientation(orientation: SplitterOrientation) -> Self {
        Self {
            orientation,
            initial_ratio: 0.5,
            spacing: 6.0,
            ratio_bounds: 0.0..=1.0,
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub fn vertical() -> Self {
        Self::with_orientation(SplitterOrientation::Vertical)
    }

    #[inline]
    #[allow(dead_code)]
    pub fn horizontal() -> Self {
        Self::with_orientation(SplitterOrientation::Horizontal)
    }

    #[inline]
    #[allow(dead_code)]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    #[inline]
    #[allow(dead_code)]
    pub fn initial_ratio(mut self, initial_ratio: f32) -> Self {
        debug_assert!((0.0..=1.0).contains(&initial_ratio));
        self.initial_ratio = initial_ratio;
        self
    }

    #[inline]
    pub fn ratio_bounds(mut self, ratio_bounds: RangeInclusive<f32>) -> Self {
        debug_assert!((0.0..=1.0).contains(ratio_bounds.start()));
        debug_assert!((0.0..=1.0).contains(ratio_bounds.end()));
        debug_assert!(ratio_bounds.start() <= ratio_bounds.end());
        self.ratio_bounds = ratio_bounds;
        self
    }

    #[inline]
    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui, &mut Ui) -> R,
    ) -> SplitterResponse<R> {
        self.show_dyn(ui, Box::new(add_contents))
    }

    pub fn show_dyn<'c, R>(
        self,
        ui: &mut Ui,
        add_contents: Box<dyn FnOnce(&mut Ui, &mut Ui) -> R + 'c>,
    ) -> SplitterResponse<R> {
        let Self {
            orientation,
            initial_ratio,
            spacing,
            ratio_bounds,
        } = self;

        let id = ui.id().with(module_path!());
        let ratio = ui
            .ctx()
            .data_mut(|d| d.get_temp::<f32>(id).unwrap_or(initial_ratio));

        let (rect, response) =
            ui.allocate_exact_size(ui.available_size_before_wrap(), Sense::hover());

        let splitter_rect = match orientation {
            SplitterOrientation::Horizontal => {
                let center = rect.min.y + rect.height() * ratio;
                Rect {
                    min: pos2(rect.min.x, center - spacing),
                    max: pos2(rect.max.x, center + spacing),
                }
            }
            SplitterOrientation::Vertical => {
                let center = rect.min.x + rect.width() * ratio;
                Rect {
                    min: pos2(center - spacing, rect.min.y),
                    max: pos2(center + spacing, rect.max.y),
                }
            }
        };

        let top_left_rect = match orientation {
            SplitterOrientation::Horizontal => {
                let mut rect = rect;
                rect.max.y = splitter_rect.min.y - ui.style().spacing.item_spacing.y;
                rect
            }
            SplitterOrientation::Vertical => {
                let mut rect = rect;
                rect.max.x = splitter_rect.min.x - ui.style().spacing.item_spacing.x;
                rect
            }
        };

        let bottom_right_rect = match orientation {
            SplitterOrientation::Horizontal => {
                let mut rect = rect;
                rect.min.y = splitter_rect.max.y + ui.style().spacing.item_spacing.y;
                rect
            }
            SplitterOrientation::Vertical => {
                let mut rect = rect;
                rect.min.x = splitter_rect.max.x + ui.style().spacing.item_spacing.x;
                rect
            }
        };

        let (line_pos_1, line_pos_2) = match orientation {
            SplitterOrientation::Horizontal => {
                (splitter_rect.left_center(), splitter_rect.right_center())
            }
            SplitterOrientation::Vertical => {
                (splitter_rect.center_top(), splitter_rect.center_bottom())
            }
        };

        let splitter_response = ui.interact(splitter_rect, id, Sense::drag());

        let drag_delta = splitter_response.drag_delta() / rect.size();
        let drag_delta = match orientation {
            SplitterOrientation::Horizontal => drag_delta.y,
            SplitterOrientation::Vertical => drag_delta.x,
        };

        let ratio = ratio + drag_delta;
        let ratio = ratio.clamp(*ratio_bounds.start(), *ratio_bounds.end());

        ui.ctx().data_mut(|d| d.insert_temp(id, ratio));

        let line_pos_1 = line_pos_1.round_to_pixels(ui.pixels_per_point());
        let line_pos_2 = line_pos_2.round_to_pixels(ui.pixels_per_point());

        let cursor_icon = match orientation {
            SplitterOrientation::Horizontal => CursorIcon::ResizeVertical,
            SplitterOrientation::Vertical => CursorIcon::ResizeHorizontal,
        };

        let visuals = if splitter_response.dragged() {
            ui.output_mut(|o| o.cursor_icon = cursor_icon);
            &ui.visuals().widgets.active
        } else if splitter_response.hovered() {
            ui.output_mut(|o| o.cursor_icon = cursor_icon);
            &ui.visuals().widgets.hovered
        } else {
            &ui.visuals().widgets.noninteractive
        };

        ui.painter()
            .line_segment([line_pos_1, line_pos_2], visuals.bg_stroke);

        let mut top_left_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(top_left_rect)
                .layout(Layout::top_down(Align::Min))
                .id_salt("top_left"),
        );

        let mut bottom_right_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(bottom_right_rect)
                .layout(Layout::top_down(Align::Min))
                .id_salt("bottom_right"),
        );

        let inner = add_contents(&mut top_left_ui, &mut bottom_right_ui);

        let inner = InnerResponse::new(inner, response);

        SplitterResponse {
            splitter_response,
            top_left_response: ui.interact(top_left_rect, top_left_ui.id(), Sense::hover()),

            bottom_right_response: ui.interact(
                bottom_right_rect,
                bottom_right_ui.id(),
                Sense::hover(),
            ),
            inner,
            rect,
        }
    }
}
