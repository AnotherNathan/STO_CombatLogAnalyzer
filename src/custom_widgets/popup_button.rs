use std::hash::Hash;

use eframe::{
    egui::{Area, Frame, Id, InnerResponse, Layout, Order, Ui, WidgetText},
    emath::Align,
    epaint::Rect,
};

pub struct PopupButton {
    title: WidgetText,
    id: Option<Id>,
}

#[derive(Default, Clone, Copy, Debug)]
struct PopupButtonState {
    open: bool,
}

impl PopupButton {
    pub fn new(title: impl Into<WidgetText>) -> Self {
        let title = title.into();
        Self { title, id: None }
    }

    pub fn with_id_source(mut self, source: impl Hash) -> Self {
        self.id = Some(Id::new(source));
        self
    }

    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<Option<R>> {
        let Self { title, id } = self;
        let id = id.unwrap_or(ui.id()).with(module_path!());
        let mut state = PopupButtonState::load(ui, id);

        let button_response = ui.button(title);
        if button_response.clicked() {
            state.open = true;
        }

        if !state.open {
            state.store(ui, id);
            return InnerResponse::new(None, button_response);
        }

        let inner = Area::new(id)
            .order(Order::Foreground)
            .default_pos(button_response.rect.min)
            .movable(true)
            .interactable(true)
            .drag_bounds(Rect::EVERYTHING)
            .show(ui.ctx(), |ui| {
                Frame::menu(ui.style())
                    .show(ui, |ui| {
                        ui.with_layout(Layout::top_down_justified(Align::LEFT), add_contents)
                            .inner
                    })
                    .inner
            });

        if !button_response.clicked() && inner.response.clicked_elsewhere() {
            // TODO find a way not to close when something inside was clicked (e.g. a combo box)
            state.open = false;
        }

        state.store(ui, id);
        InnerResponse::new(Some(inner.inner), button_response)
    }
}

impl PopupButtonState {
    fn load(ui: &mut Ui, id: Id) -> Self {
        ui.ctx()
            .data()
            .get_temp::<PopupButtonState>(id)
            .unwrap_or_default()
    }

    fn store(self, ui: &mut Ui, id: Id) {
        ui.ctx().data().insert_temp(id, self);
    }
}
