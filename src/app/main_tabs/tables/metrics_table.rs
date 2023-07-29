use std::cmp::Reverse;

use educe::Educe;
use eframe::egui::*;

use crate::{
    analyzer::*,
    app::main_tabs::common::*,
    custom_widgets::table::*,
    helpers::{number_formatting::NumberFormatter, F64TotalOrd},
};

#[macro_export]
macro_rules! col {
    ($name:expr, $sort:expr, $show:expr $(,)?) => {
        ColumnDescriptor {
            name: $name,
            name_info: None,
            sort: $sort,
            show: $show,
        }
    };

    ($name:expr, $name_info:expr, $sort:expr, $show:expr $(,)?) => {
        ColumnDescriptor {
            name: $name,
            name_info: Some($name_info),
            sort: $sort,
            show: $show,
        }
    };
}

pub struct MetricsTable<T: 'static> {
    columns: &'static [ColumnDescriptor<T>],
    players: Vec<MetricsTablePart<T>>,
    selected_id: Option<u32>,
}

#[derive(Educe)]
#[educe(Deref, DerefMut)]
pub struct MetricsTablePart<T> {
    #[educe(Deref, DerefMut)]
    pub data: T,
    pub name: String,
    id: u32,

    pub sub_parts: Vec<Self>,

    open: bool,
}

pub enum TableSelection<'a, T> {
    SubPartsOrSingle(&'a MetricsTablePart<T>),
    Single(&'a MetricsTablePart<T>),
    Unselect,
}

#[derive(Copy)]
pub struct ColumnDescriptor<T: 'static> {
    pub name: &'static str,
    pub name_info: Option<&'static str>,
    pub sort: fn(&mut MetricsTable<T>),
    pub show: fn(&mut MetricsTablePart<T>, &mut TableRow),
}

impl<T> Clone for ColumnDescriptor<T> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            name_info: self.name_info.clone(),
            sort: self.sort.clone(),
            show: self.show.clone(),
        }
    }
}

impl<T: 'static> MetricsTable<T> {
    pub fn empty_base(columns: &'static [ColumnDescriptor<T>]) -> Self {
        Self {
            players: Vec::new(),
            selected_id: None,
            columns,
        }
    }

    pub fn new_base<G: AnalysisGroup>(
        columns: &'static [ColumnDescriptor<T>],
        combat: &Combat,
        mut group: impl FnMut(&Player) -> &G,
        data_new: fn(&G, &Combat, &mut NumberFormatter) -> T,
    ) -> Self {
        let mut number_formatter = NumberFormatter::new();
        let mut id_source = 0;
        let mut table = Self {
            columns,
            players: combat
                .players
                .values()
                .map(|p| {
                    MetricsTablePart::new(
                        group(p),
                        combat,
                        &mut number_formatter,
                        &mut id_source,
                        data_new,
                    )
                })
                .collect(),
            selected_id: None,
        };
        (table.columns[0].sort)(&mut table);

        table
    }

    pub fn show(&mut self, ui: &mut Ui, mut on_selected: impl FnMut(TableSelection<T>)) {
        ScrollArea::horizontal().show(ui, |ui| {
            Table::new(ui)
                .cell_spacing(10.0)
                .header(HEADER_HEIGHT, |mut r| {
                    r.cell(|ui| {
                        ui.label("Name");
                    });

                    for column in self.columns.iter() {
                        self.show_column_header(&mut r, column);
                    }
                })
                .body(ROW_HEIGHT, |mut t| {
                    for player in self.players.iter_mut() {
                        player.show(
                            &self.columns,
                            &mut t,
                            0.0,
                            &mut self.selected_id,
                            &mut on_selected,
                        );
                    }
                });
        });
    }

    fn show_column_header(&mut self, row: &mut TableRow, column: &ColumnDescriptor<T>) {
        let response = row.selectable_cell(false, |ui| {
            ui.label(column.name);
        });
        if response.clicked() {
            (column.sort)(self);
        }
        if let Some(info) = column.name_info {
            response.on_hover_text(info);
        }
    }

    pub fn sort_by_option_f64_desc(
        &mut self,
        mut key: impl FnMut(&MetricsTablePart<T>) -> Option<f64> + Copy,
    ) {
        self.sort_by_desc(move |p| key(p).map(|v| F64TotalOrd(v)));
    }

    pub fn sort_by_option_f64_asc(
        &mut self,
        mut key: impl FnMut(&MetricsTablePart<T>) -> Option<f64> + Copy,
    ) {
        self.sort_by_asc(move |p| key(p).map(|v| F64TotalOrd(v)));
    }

    pub fn sort_by_desc<K: Ord>(&mut self, mut key: impl FnMut(&MetricsTablePart<T>) -> K + Copy) {
        self.players.sort_unstable_by_key(|p| Reverse(key(p)));

        self.players.iter_mut().for_each(|p| p.sort_by_desc(key));
    }

    pub fn sort_by_asc<K: Ord>(&mut self, key: impl FnMut(&MetricsTablePart<T>) -> K + Copy) {
        self.players.sort_unstable_by_key(key);

        self.players.iter_mut().for_each(|p| p.sort_by_asc(key));
    }
}

impl<T> MetricsTablePart<T> {
    fn new<G: AnalysisGroup>(
        source: &G,
        combat: &Combat,
        number_formatter: &mut NumberFormatter,
        id_source: &mut u32,
        data_new: fn(&G, &Combat, &mut NumberFormatter) -> T,
    ) -> Self {
        let id = *id_source;
        *id_source += 1;
        let sub_parts = source
            .sub_groups()
            .values()
            .map(|s| MetricsTablePart::new(s, combat, number_formatter, id_source, data_new))
            .collect();

        Self {
            data: data_new(source, combat, number_formatter),
            name: source.name().get(&combat.name_manager).to_string(),
            id,
            sub_parts,
            open: false,
        }
    }

    fn show(
        &mut self,
        columns: &[ColumnDescriptor<T>],
        table: &mut TableBody,
        indent: f32,
        selected_id: &mut Option<u32>,
        on_selected: &mut impl FnMut(TableSelection<T>),
    ) {
        let response = table.selectable_row(Some(self.id) == *selected_id, |mut r| {
            r.cell(|ui| {
                ui.horizontal(|ui| {
                    ui.add_space(indent * 30.0);
                    let symbol = if self.open { "⏷" } else { "⏵" };
                    let can_open = self.sub_parts.len() > 0;
                    if ui
                        .add_visible(can_open, SelectableLabel::new(false, symbol))
                        .clicked()
                    {
                        self.open = !self.open;
                    }

                    ui.label(&self.name);
                });
            });

            for column in columns.iter() {
                (column.show)(self, &mut r);
            }
        });

        if response.clicked() {
            if *selected_id == Some(self.id) {
                *selected_id = None;
                on_selected(TableSelection::Unselect);
            } else {
                *selected_id = Some(self.id);
                on_selected(TableSelection::SubPartsOrSingle(self));
            }
        }

        response.context_menu(|ui| {
            if ui
                .selectable_label(false, "copy name to clipboard")
                .clicked()
            {
                ui.output_mut(|o| o.copied_text = self.name.clone());
                ui.close_menu();
            }

            if ui
                .selectable_label(false, "show diagrams for this")
                .clicked()
            {
                *selected_id = Some(self.id);
                on_selected(TableSelection::Single(self));
                ui.close_menu();
            }
        });

        if self.open {
            for sub_part in self.sub_parts.iter_mut() {
                sub_part.show(columns, table, indent + 1.0, selected_id, on_selected);
            }
        }
    }

    pub fn sort_by_desc<K: Ord>(&mut self, mut key: impl FnMut(&Self) -> K + Copy) {
        self.sub_parts.sort_unstable_by_key(|p| Reverse(key(p)));

        self.sub_parts.iter_mut().for_each(|p| p.sort_by_desc(key));
    }

    pub fn sort_by_asc<K: Ord>(&mut self, key: impl FnMut(&Self) -> K + Copy) {
        self.sub_parts.sort_unstable_by_key(key);

        self.sub_parts.iter_mut().for_each(|p| p.sort_by_asc(key));
    }
}
