use std::{thread::JoinHandle, time::Duration};

use eframe::egui::*;
use reqwest::{blocking::ClientBuilder, Url};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    custom_widgets::{number_edit::NumberEdit, table::Table},
    helpers::number_formatting::NumberFormatter,
};

use super::common::{spawn_request, RequestError};

const PAGE_SIZE: i32 = 50;

#[derive(Default)]
pub struct Records {
    state: LaddersState,
}

impl Records {
    pub fn show(&mut self, ui: &mut Ui, url: &str) {
        if ui.selectable_label(self.state.show(), "Records").clicked() {
            self.state = Self::begin_load_ladders(ui.ctx().clone(), url);
        }

        let url = match Url::parse(url) {
            Ok(u) => u,
            Err(_) => {
                ui.label("the provided upload URL is invalid");
                return;
            }
        };

        let mut open = self.state.show();
        Window::new("Records")
            .collapsible(false)
            .constrain(true)
            .open(&mut open)
            .default_size([1280.0, 720.0])
            .max_size(ui.ctx().screen_rect().size() - vec2(120.0, 120.0))
            .show(ui.ctx(), |ui| match &mut self.state {
                LaddersState::Collapsed => (),
                LaddersState::Loading(join_handle) => {
                    if join_handle.as_ref().unwrap().is_finished() {
                        self.state = join_handle.take().unwrap().join().unwrap();
                        ui.ctx().request_repaint_of(ViewportId::ROOT);
                    }

                    Self::show_loading_ladders(ui);
                }
                LaddersState::Loaded(state) => Self::show_ladders(ui, url, state),
                LaddersState::LoadError(err) => {
                    ui.label(&*err);
                }
            });

        if !open {
            self.state = LaddersState::Collapsed;
        }
    }

    fn show_loading_ladders(ui: &mut Ui) {
        ui.add_space(20.0);
        ui.label("loading record tables...");
        ui.add_space(40.0);
        ui.label(WidgetText::from("⏳").color(Color32::YELLOW));
        ui.add_space(20.0);
    }

    fn show_ladders(ui: &mut Ui, url: Url, state: &mut LaddersLoadedState) {
        if Self::show_ladders_combo_box(ui, &mut state.selected_ladder, &state.ladders) {
            state.entries_state = Self::begin_load_ladder_entries(
                ui.ctx().clone(),
                url.clone(),
                state.ladders.ladders[state.selected_ladder].clone(),
                1,
                state.search_player.clone(),
            );
        }
        ui.horizontal(|ui| {
            let mut search = TextEdit::singleline(&mut state.search_player)
                .desired_width(400.0)
                .hint_text("search for Player")
                .show(ui)
                .response
                .lost_focus()
                && ui.input(|i| i.key_pressed(Key::Enter));
            search |= ui.button("Search").clicked();
            if search {
                state.entries_state = Self::begin_load_ladder_entries(
                    ui.ctx().clone(),
                    url.clone(),
                    state.ladders.ladders[state.selected_ladder].clone(),
                    1,
                    state.search_player.clone(),
                );
            }
        });
        ui.separator();
        Self::show_entries(
            ui,
            url,
            &state.ladders.ladders[state.selected_ladder],
            &state.search_player,
            &mut state.entries_state,
        );
    }

    fn show_ladders_combo_box(ui: &mut Ui, selected_ladder: &mut usize, ladders: &Ladders) -> bool {
        ComboBox::new("ladders", "Record Tables")
            .selected_text(&ladders.ladders[*selected_ladder].name)
            .width(400.0)
            .show_ui(ui, |ui| {
                ladders.ladders.iter().enumerate().any(|(index, ladder)| {
                    ui.selectable_value(selected_ladder, index, &ladder.name)
                        .changed()
                })
            })
            .inner
            .unwrap_or(false)
    }

    fn show_entries(
        ui: &mut Ui,
        url: Url,
        selected_ladder: &Ladder,
        search_player: &String,
        state: &mut LadderEntriesState,
    ) {
        match state {
            LadderEntriesState::Loading(join_handle) => {
                if join_handle.as_ref().unwrap().is_finished() {
                    let join_handle = join_handle.take().unwrap();
                    *state = join_handle.join().unwrap();
                    ui.ctx().request_repaint_of(ViewportId::ROOT);
                }

                ui.add_space(20.0);
                ui.label("loading table entries...");
                ui.add_space(40.0);
                ui.label(WidgetText::from("⏳").color(Color32::YELLOW));
                ui.add_space(20.0);
            }
            LadderEntriesState::Loaded(loaded_state) => {
                let mut change_page = None;
                ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                    ui.label("Page:");
                    ui.add_enabled_ui(loaded_state.entries.page > 1, |ui| {
                        if ui.button("⏴").clicked() {
                            change_page = Some(loaded_state.entries.page - 1);
                        }
                    });
                    if NumberEdit::new(&mut loaded_state.page, "page edit")
                        .clamp_min(1)
                        .clamp_max(loaded_state.entries.page_count)
                        .desired_text_edit_width(40.0)
                        .show(ui)
                        .lost_focus()
                        && loaded_state.page != loaded_state.entries.page
                    {
                        change_page = Some(loaded_state.page);
                    }
                    ui.add_enabled_ui(
                        loaded_state.entries.page < loaded_state.entries.page_count,
                        |ui| {
                            if ui.button("⏵").clicked() {
                                change_page = Some(loaded_state.entries.page + 1);
                            }
                        },
                    );
                });
                Self::show_entries_table(ui, loaded_state);
                if let Some(change_page) = change_page {
                    *state = Self::begin_load_ladder_entries(
                        ui.ctx().clone(),
                        url,
                        selected_ladder.clone(),
                        change_page,
                        search_player.clone(),
                    );
                }
            }
            LadderEntriesState::LoadError(err) => {
                ui.label(&*err);
            }
        }
    }

    fn show_entries_table(ui: &mut Ui, loaded_state: &mut LoadedEntriesState) {
        if loaded_state.entries.entries.len() == 0 {
            ui.label("no entries");
            return;
        }

        ScrollArea::horizontal().show(ui, |ui| {
            Table::new(ui)
                .header(15.0, |r| {
                    for header in loaded_state.entries.data_headers.iter() {
                        r.cell(|ui| {
                            ui.label(&*header);
                        });
                    }
                })
                .body(25.0, |b| {
                    for (index, entry) in loaded_state.entries.entries.iter().enumerate() {
                        if b.selectable_row(loaded_state.selected_row == Some(index), |r| {
                            for data in entry.data.iter() {
                                if data.is_number {
                                    r.cell_with_layout(
                                        Layout::right_to_left(Align::Center),
                                        |ui| {
                                            ui.label(&data.value);
                                        },
                                    );
                                } else {
                                    r.cell(|ui| {
                                        ui.label(&data.value);
                                    });
                                }
                            }
                        })
                        .clicked()
                        {
                            if loaded_state.selected_row == Some(index) {
                                loaded_state.selected_row = None
                            } else {
                                loaded_state.selected_row = Some(index)
                            }
                        }
                    }
                });
        });
    }

    fn begin_load_ladders(ctx: Context, url: &str) -> LaddersState {
        let url = match Url::parse(url) {
            Ok(u) => u,
            Err(_) => {
                return LaddersState::LoadError("the provided upload URL is invalid".into());
            }
        };

        let join_handle = spawn_request(move || Self::load_ladders(ctx, url));
        LaddersState::Loading(Some(join_handle))
    }

    fn load_ladders(ctx: Context, url: Url) -> LaddersState {
        let state = match Self::do_load_ladders(url.clone()) {
            Ok(ladders) => {
                if ladders.ladders.len() == 0 {
                    return LaddersState::LoadError("Failed to load records tables.".into());
                }
                let ladder = ladders.ladders.first().unwrap();
                LaddersState::Loaded(LaddersLoadedState {
                    entries_state: Self::begin_load_ladder_entries(
                        ctx.clone(),
                        url,
                        ladder.clone(),
                        1,
                        String::new(),
                    ),
                    ladders,
                    selected_ladder: 0,
                    search_player: String::new(),
                })
            }
            Err(err) => LaddersState::LoadError(format!(
                "{}",
                err.action_error("Failed to load records tables.")
            )),
        };
        ctx.request_repaint_after_for(Duration::from_millis(10), ViewportId::ROOT);
        state
    }

    fn do_load_ladders(mut url: Url) -> Result<Ladders, RequestError> {
        let client = ClientBuilder::new().build().unwrap();
        url.set_path("/ladder/");
        let response = client
            .get(url)
            .query(&[("page_size", &i32::MAX.to_string())])
            .send()?;
        if !response.status().is_success() {
            return Err(RequestError::from(response));
        }
        let ladders = response.json::<LaddersModel>()?;
        Ok(ladders.into())
    }

    fn begin_load_ladder_entries(
        ctx: Context,
        url: Url,
        ladder: Ladder,
        page: i32,
        player: String,
    ) -> LadderEntriesState {
        let join_handle =
            spawn_request(move || Self::load_ladder_entries(ctx, url, ladder, page, player));
        LadderEntriesState::Loading(Some(join_handle))
    }

    fn load_ladder_entries(
        ctx: Context,
        url: Url,
        ladder: Ladder,
        page: i32,
        player: String,
    ) -> LadderEntriesState {
        let state = match Self::do_load_ladder_entries(url, ladder, page, player) {
            Ok(entries) => LadderEntriesState::Loaded(LoadedEntriesState {
                page: entries.page,
                selected_row: None,
                entries,
            }),
            Err(err) => LadderEntriesState::LoadError(format!(
                "{}",
                err.action_error("Failed to load record table entries.")
            )),
        };
        ctx.request_repaint_after_for(Duration::from_millis(10), ViewportId::ROOT);
        state
    }

    fn do_load_ladder_entries(
        mut url: Url,
        ladder: Ladder,
        page: i32,
        player: String,
    ) -> Result<LadderEntries, RequestError> {
        let client = ClientBuilder::new().build().unwrap();
        url.set_path("/ladder-entries/");
        let ladder_id = ladder.id.to_string();
        let page_size = PAGE_SIZE.to_string();
        let ordering = format!("-data__{}", ladder.metric);
        let page_str = page.to_string();
        let mut query = vec![
            ("ladder", ladder_id.as_str()),
            ("page_size", &page_size),
            ("ordering", &ordering),
            ("page", &page_str),
        ];

        if !player.is_empty() {
            query.push(("player__icontains", &player)); // i for case insensitive
        }
        let response = client.get(url).query(&query).send()?;
        if !response.status().is_success() {
            return Err(RequestError::from(response));
        }
        let ladder_entries = response.json::<LadderEntriesModel>()?;
        Ok(LadderEntries::new(page, ladder_entries))
    }
}

#[derive(Default)]
enum LaddersState {
    #[default]
    Collapsed,
    Loading(Option<JoinHandle<Self>>),
    Loaded(LaddersLoadedState),
    LoadError(String),
}

struct LaddersLoadedState {
    ladders: Ladders,
    selected_ladder: usize,
    entries_state: LadderEntriesState,
    search_player: String,
}

impl LaddersState {
    fn show(&self) -> bool {
        match self {
            LaddersState::Collapsed => false,
            _ => true,
        }
    }
}

enum LadderEntriesState {
    Loading(Option<JoinHandle<Self>>),
    Loaded(LoadedEntriesState),
    LoadError(String),
}

struct LoadedEntriesState {
    page: i32,
    selected_row: Option<usize>,
    entries: LadderEntries,
}

#[derive(Deserialize, Debug)]
struct LaddersModel {
    results: Vec<LadderModel>,
}

#[derive(Deserialize, Debug, Clone)]
struct LadderModel {
    id: i32,
    name: String,
    difficulty: String,
    metric: String,
    is_solo: bool,
}

#[derive(Deserialize, Debug)]
struct LadderEntriesModel {
    count: i32,
    results: Vec<LadderEntryModel>,
}

#[derive(Deserialize, Debug)]
struct LadderEntryModel {
    date: String,
    player: String,
    rank: i32,
    data: serde_json::Map<String, serde_json::Value>,
}

struct Ladders {
    ladders: Vec<Ladder>,
}

impl From<LaddersModel> for Ladders {
    fn from(value: LaddersModel) -> Self {
        Self {
            ladders: value.results.into_iter().map(|l| l.into()).collect(),
        }
    }
}

#[derive(Clone)]
struct Ladder {
    id: i32,
    metric: String,
    name: String,
}

impl From<LadderModel> for Ladder {
    fn from(value: LadderModel) -> Self {
        Self {
            name: if value.is_solo {
                format!(
                    "[Solo] {} ({}) - {}",
                    value.name, value.difficulty, value.metric
                )
            } else {
                format!("{} ({}) - {}", value.name, value.difficulty, value.metric)
            },
            id: value.id,
            metric: value.metric,
        }
    }
}

struct LadderEntries {
    page_count: i32,
    page: i32,
    data_headers: Vec<String>,
    entries: Vec<LadderEntry>,
}

impl LadderEntries {
    fn new(page: i32, model: LadderEntriesModel) -> Self {
        let mut formatter = NumberFormatter::new();
        Self {
            page_count: model.count / PAGE_SIZE + if model.count % PAGE_SIZE > 0 { 1 } else { 0 },
            page,
            data_headers: model
                .results
                .first()
                .map(|e| {
                    ["Rank".to_owned(), "Player".to_owned()]
                        .into_iter()
                        .chain(e.data.keys().cloned().map(|h| h.replace('_', " ")))
                        .chain(std::iter::once("Date".to_owned()))
                        .collect()
                })
                .unwrap_or(Vec::new()),
            entries: model
                .results
                .into_iter()
                .map(|e| LadderEntry::new(e, &mut formatter))
                .collect(),
        }
    }
}

struct LadderEntry {
    data: Vec<DataValue>,
}

impl LadderEntry {
    fn new(model: LadderEntryModel, formatter: &mut NumberFormatter) -> Self {
        Self {
            data: [
                DataValue::number(model.rank.to_string()),
                DataValue::non_number(model.player),
            ]
            .into_iter()
            .chain(model.data.values().map(|value| {
                match value {
                    Value::Null => DataValue::non_number(String::new()),
                    Value::Bool(bool) => {
                        DataValue::non_number(if *bool { "✔" } else { "✖" }.into())
                    }
                    Value::Number(number) => DataValue::number(
                        if number.is_f64() {
                            formatter.format(number.as_f64().unwrap(), 2)
                        } else {
                            number.to_string()
                        }
                        .into(),
                    ),
                    Value::String(str) => DataValue::non_number(str.into()),
                    Value::Array(array) => DataValue::non_number(format!("{:?}", array).into()),
                    Value::Object(object) => DataValue::non_number(format!("{:?}", object).into()),
                }
            }))
            .chain(std::iter::once(DataValue::non_number(model.date)))
            .collect(),
        }
    }
}

struct DataValue {
    value: String,
    is_number: bool,
}

impl DataValue {
    fn number(value: String) -> Self {
        Self {
            value,
            is_number: true,
        }
    }

    fn non_number(value: String) -> Self {
        Self {
            value,
            is_number: false,
        }
    }
}
