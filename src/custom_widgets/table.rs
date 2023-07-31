use std::f32::INFINITY;

use eframe::egui::*;

pub struct Table<'a> {
    ui: &'a mut Ui,
    id: Id,
    min_scroll_height: f32,
    max_scroll_height: f32,
    cell_spacing: f32,
    striped: bool,
}

pub struct TableWithHeader<'a> {
    table: Table<'a>,
    state: State,
    header_rect: Rect,
}

pub struct TableBody<'a> {
    ui: &'a mut Ui,
    row_height: f32,
    cell_spacing: f32,
    striped: bool,
    state: &'a mut State,
    current_row: usize,
    left_top: Pos2,
}

pub struct TableRow<'a> {
    ui: &'a mut Ui,
    state: &'a mut State,
    current_column: usize,
    left_top: Pos2,
    left_offset: f32,
    row_height: f32,
    cell_spacing: f32,
}

#[derive(Debug, Default, Clone)]
struct State {
    columns: Vec<ColumnState>,
    size: Vec2,
    last_size: Vec2,
}

#[derive(Debug, Default, Clone)]
struct ColumnState {
    size: f32,
    last_size: f32,
}

#[allow(dead_code)]
impl<'a> Table<'a> {
    pub fn new(ui: &'a mut Ui) -> Self {
        let id = ui.id().with(module_path!());
        Self {
            ui,
            id,
            min_scroll_height: 0.0,
            max_scroll_height: INFINITY,
            cell_spacing: 5.0,
            striped: true,
        }
    }

    pub fn id(mut self, id: impl Into<Id>) -> Self {
        self.id = id.into();
        self
    }

    pub fn min_scroll_height(mut self, min_scroll_height: f32) -> Self {
        self.min_scroll_height = min_scroll_height;
        self
    }

    pub fn max_scroll_height(mut self, max_scroll_height: f32) -> Self {
        self.max_scroll_height = max_scroll_height;
        self
    }

    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    pub fn cell_spacing(mut self, cell_spacing: f32) -> Self {
        self.cell_spacing = cell_spacing;
        self
    }

    pub fn header(
        self,
        header_height: f32,
        add_header: impl FnOnce(&mut TableRow),
    ) -> TableWithHeader<'a> {
        let left_top = self.ui.cursor().left_top();
        let mut state = State::load(&self.ui, self.id);
        TableRow::show(
            self.ui,
            &mut state,
            0,
            left_top,
            header_height,
            self.cell_spacing,
            add_header,
            false,
            None,
        );
        let header_rect = Rect::from_min_size(left_top, vec2(state.last_size.x, header_height));
        self.ui.allocate_rect(header_rect, Sense::hover());

        TableWithHeader {
            table: self,
            state,
            header_rect,
        }
    }

    pub fn body(self, row_height: f32, add_body: impl FnOnce(&mut TableBody)) {
        let state = State::load(&self.ui, self.id);

        self.body_inner(row_height, add_body, state, None);
    }

    fn body_inner(
        self,
        row_height: f32,
        add_body: impl FnOnce(&mut TableBody),
        mut state: State,
        header_rect: Option<Rect>,
    ) {
        let Self {
            ui,
            id,
            min_scroll_height,
            max_scroll_height,
            striped,
            cell_spacing,
        } = self;
        let scroll_output = ScrollArea::vertical()
            .id_source(id.with("__table_scroll"))
            .min_scrolled_height(min_scroll_height)
            .max_height(max_scroll_height)
            .show(ui, |ui| {
                let left_top = ui.cursor().left_top();
                let mut body = TableBody {
                    current_row: 0,
                    left_top,
                    row_height,
                    cell_spacing,
                    striped,
                    state: &mut state,
                    ui,
                };

                add_body(&mut body);

                let rect = Rect::from_min_size(left_top, state.last_size);
                ui.allocate_rect(rect, Sense::hover());
                rect
            });

        let rect = scroll_output.inner.intersect(scroll_output.inner_rect);
        let separators_rect = header_rect.map(|h| h.union(rect)).unwrap_or(rect);
        ColumnState::draw_separators(&state.columns, ui, separators_rect, cell_spacing);
        if state.finish(ui, id) {
            ui.ctx().request_repaint();
        }
    }
}

impl<'a> TableWithHeader<'a> {
    pub fn body(self, row_height: f32, add_body: impl FnOnce(&mut TableBody)) {
        let Self {
            table,
            state,
            header_rect,
        } = self;
        table.body_inner(row_height, add_body, state, Some(header_rect));
    }
}

impl<'a> TableBody<'a> {
    pub fn row(&mut self, add_cells: impl FnOnce(&mut TableRow)) -> Response {
        let response = TableRow::show(
            self.ui,
            &mut self.state,
            self.current_row,
            self.left_top,
            self.row_height,
            self.cell_spacing,
            add_cells,
            self.striped && (self.current_row % 2) == 0,
            None,
        );

        self.current_row += 1;

        response
    }

    pub fn selectable_row(
        &mut self,
        checked: bool,
        add_cells: impl FnOnce(&mut TableRow),
    ) -> Response {
        let response = TableRow::show(
            self.ui,
            &mut self.state,
            self.current_row,
            self.left_top,
            self.row_height,
            self.cell_spacing,
            add_cells,
            self.striped && (self.current_row % 2) == 0,
            Some(checked),
        );

        self.current_row += 1;

        response
    }
}

impl<'a> TableRow<'a> {
    fn show(
        ui: &mut Ui,
        state: &mut State,
        row_index: usize,
        table_left_top: Pos2,
        row_height: f32,
        cell_spacing: f32,
        add_cells: impl FnOnce(&mut TableRow),
        is_stripe: bool,
        checked: Option<bool>,
    ) -> Response {
        let left_top = pos2(
            table_left_top.x,
            table_left_top.y + row_index as f32 * row_height,
        );
        let rect = Rect::from_min_size(left_top, vec2(state.last_size.x, row_height));
        let sense = if checked.is_some() {
            Sense::click()
        } else {
            Sense::hover()
        };
        let response = ui.interact(rect, ui.id().with(row_index), sense);

        draw_visuals(ui, is_stripe, checked, &response);

        let mut row = TableRow {
            current_column: 0,
            state: state,
            ui,
            left_top,
            left_offset: 0.0,
            row_height: row_height,
            cell_spacing,
        };
        add_cells(&mut row);
        state.update_height(row_index + 1, row_height);

        response
    }

    pub fn cell(&mut self, add_column: impl FnOnce(&mut Ui)) -> Response {
        self.cell_with_layout(Layout::left_to_right(Align::Center), add_column)
    }

    pub fn cell_with_layout(
        &mut self,
        layout: Layout,
        add_column: impl FnOnce(&mut Ui),
    ) -> Response {
        self.show_cell(layout, add_column, Sense::hover(), None)
    }

    pub fn selectable_cell(&mut self, checked: bool, add_column: impl FnOnce(&mut Ui)) -> Response {
        self.selectable_cell_with_layout(checked, Layout::left_to_right(Align::Center), add_column)
    }

    pub fn selectable_cell_with_layout(
        &mut self,
        checked: bool,
        layout: Layout,
        add_column: impl FnOnce(&mut Ui),
    ) -> Response {
        self.show_cell(layout, add_column, Sense::click(), Some(checked))
    }

    fn show_cell(
        &mut self,
        layout: Layout,
        add_column: impl FnOnce(&mut Ui),
        sense: Sense,
        checked: Option<bool>,
    ) -> Response {
        if self.state.columns.len() <= self.current_column {
            self.state.columns.push(Default::default());
        }

        let column = &mut self.state.columns[self.current_column];

        self.left_offset += self.cell_spacing;

        let rect = Rect::from_min_size(
            self.left_top + vec2(self.left_offset, 0.0),
            vec2(column.last_size, self.row_height),
        );
        let interact_rect = rect.expand2(vec2(self.cell_spacing, 0.0));
        let response = self
            .ui
            .interact(interact_rect, self.ui.next_auto_id(), sense);
        draw_visuals(self.ui, false, checked, &response);
        let mut ui = self.ui.child_ui(rect, layout);

        add_column(&mut ui);

        let content_rect = ui.min_rect();

        self.current_column += 1;
        self.left_offset += column.last_size + self.cell_spacing;
        column.update(content_rect.width());
        self.state.update_width(self.left_offset);
        response
    }
}

impl ColumnState {
    fn update(&mut self, cell_width: f32) {
        self.size = self.size.max(cell_width);
    }

    fn finish(&mut self) -> bool {
        let repaint_required = (self.last_size - self.size).abs() > 0.5;
        self.last_size = self.size;
        self.size = 0.0;
        repaint_required
    }

    fn draw_separators(columns: &[Self], ui: &mut Ui, rect: Rect, cell_spacing: f32) {
        if columns.len() == 0 {
            return;
        }

        let left_top = rect.left_top();
        let mut left_offset = 0.0;
        for column in columns.iter().take(columns.len() - 1) {
            left_offset += column.last_size + 2.0 * cell_spacing;
            let start = ui
                .painter()
                .round_pos_to_pixels(left_top + vec2(left_offset, 0.0));
            let end = ui
                .painter()
                .round_pos_to_pixels(start + vec2(0.0, rect.height()));
            ui.painter()
                .line_segment([start, end], ui.visuals().noninteractive().bg_stroke);
        }
    }
}

impl State {
    fn load(ui: &Ui, id: Id) -> Self {
        ui.data_mut(|d| d.get_temp(id))
            .unwrap_or_else(|| Default::default())
    }

    fn store(self, ui: &Ui, id: Id) {
        ui.data_mut(|d| d.insert_temp(id, self));
    }

    fn update_width(&mut self, row_width: f32) {
        self.size.x = self.size.x.max(row_width);
    }

    fn update_height(&mut self, rows: usize, row_height: f32) {
        self.size.y = self.size.y.max(rows as f32 * row_height);
    }

    fn finish(mut self, ui: &Ui, id: Id) -> bool {
        let size_change = (self.size - self.last_size).abs();
        let mut repaint_required = size_change.x > 0.5 || size_change.y > 0.5;
        self.last_size = self.size;
        self.size = Vec2::ZERO;

        while self.columns.last().map(|s| s.size == 0.0).unwrap_or(false) {
            self.columns.pop();
        }

        for column_size in self.columns.iter_mut() {
            repaint_required |= column_size.finish();
        }

        self.store(ui, id);

        repaint_required
    }
}

fn draw_visuals(ui: &mut Ui, is_stripe: bool, checked: Option<bool>, response: &Response) {
    match checked {
        Some(true) => {
            ui.painter().rect_filled(
                response.rect,
                0.0,
                ui.style().interact_selectable(response, true).bg_fill,
            );
        }
        Some(false) if response.hovered() => {
            ui.painter().rect_filled(
                response.rect,
                0.0,
                ui.style().interact_selectable(response, false).bg_fill,
            );
        }
        _ if is_stripe => {
            ui.painter()
                .rect_filled(response.rect, 0.0, ui.visuals().faint_bg_color);
        }
        _ => (),
    }

    if checked.is_some() && response.hovered() {
        ui.painter().rect_stroke(
            response.rect,
            0.0,
            ui.style()
                .interact_selectable(&response, checked.unwrap())
                .bg_stroke,
        );
    }
}
