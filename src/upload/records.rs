use std::{fs::File, io::Write, path::PathBuf, thread::JoinHandle, time::Duration};

use eframe::{egui::*, Frame};
use flate2::write::GzDecoder;
use itertools::Either;
use reqwest::{blocking::ClientBuilder, Url};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    custom_widgets::{
        number_edit::NumberEdit,
        table::{Table, TableRow},
    },
    helpers::number_formatting::NumberFormatter,
};

use super::common::{spawn_request, RequestError};

const PAGE_SIZE: i32 = 50;
static CHERRY_DATA_PICKS: &[&str] = &[
    "Rank",
    "Player",
    "DPS",
    "debuff",
    "combat time",
    "damage share",
];

#[derive(Default)]
pub enum Records {
    #[default]
    Collapsed,
    Loading(Option<JoinHandle<Self>>),
    #[allow(private_interfaces)]
    Loaded(LoadedLadders),
    LoadError(String),
}

impl Records {
    pub fn show(&mut self, ui: &mut Ui, frame: &Frame, url: &str) {
        let url = match Url::parse(url) {
            Ok(u) => u,
            Err(_) => {
                ui.label("the provided upload URL is invalid (change in Settings->Upload)");
                return;
            }
        };
        if ui.selectable_label(!self.collapsed(), "Records").clicked() {
            *self = Self::begin_load_ladders(ui.ctx().clone(), url.clone());
        }

        let mut open = !self.collapsed();
        Window::new("Records")
            .collapsible(false)
            .constrain(true)
            .open(&mut open)
            .default_size([1280.0, 720.0])
            .max_size(ui.ctx().screen_rect().size() - vec2(120.0, 120.0))
            .show(ui.ctx(), |ui| match self {
                Self::Collapsed => (),
                Self::Loading(join_handle) => {
                    if join_handle.as_ref().unwrap().is_finished() {
                        *self = join_handle.take().unwrap().join().unwrap();
                        ui.ctx().request_repaint_of(ViewportId::ROOT);
                    }

                    Self::show_loading_ladders(ui);
                }
                Self::Loaded(loaded_ladders) => loaded_ladders.show(ui, frame, url),
                Self::LoadError(err) => {
                    ui.label(&*err);
                }
            });

        if !open {
            *self = Self::Collapsed;
        }
    }

    fn collapsed(&self) -> bool {
        match self {
            Self::Collapsed => true,
            _ => false,
        }
    }

    fn show_loading_ladders(ui: &mut Ui) {
        ui.add_space(20.0);
        ui.label("loading record tables...");
        ui.add_space(40.0);
        ui.label(WidgetText::from("⏳").color(Color32::YELLOW));
        ui.add_space(20.0);
    }

    fn begin_load_ladders(ctx: Context, url: Url) -> Self {
        let join_handle = spawn_request(move || Self::load_ladders(ctx, url));
        Self::Loading(Some(join_handle))
    }

    fn load_ladders(ctx: Context, url: Url) -> Self {
        let state = match Self::do_load_ladders(url.clone()) {
            Ok(ladders) => {
                if ladders.results.len() == 0 {
                    return Self::LoadError("Failed to load records tables.".into());
                }
                Self::Loaded(LoadedLadders::new(ladders, &ctx, url))
            }
            Err(err) => Self::LoadError(format!(
                "{}",
                err.action_error("Failed to load records tables.")
            )),
        };
        ctx.request_repaint_after_for(Duration::from_millis(10), ViewportId::ROOT);
        state
    }

    fn do_load_ladders(mut url: Url) -> Result<LaddersModel, RequestError> {
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
        Ok(ladders)
    }
}

struct LoadedLadders {
    ladders: Vec<Ladder>,
    selected_ladder: usize,
    entries: Entries,
}

impl LoadedLadders {
    fn new(ladders: LaddersModel, ctx: &Context, url: Url) -> Self {
        let ladders: Vec<Ladder> = ladders.results.into_iter().map(|l| l.into()).collect();
        let ladder = ladders.first().unwrap();
        Self {
            entries: Entries::begin_load_ladder_entries(
                ctx.clone(),
                url,
                ladder.clone(),
                1,
                String::new(),
                false,
            ),
            selected_ladder: 0,
            ladders,
        }
    }

    fn show(&mut self, ui: &mut Ui, frame: &Frame, url: Url) {
        if self.show_ladders_combo_box(ui) {
            self.entries = Entries::begin_load_ladder_entries(
                ui.ctx().clone(),
                url.clone(),
                self.ladders[self.selected_ladder].clone(),
                1,
                String::new(),
                false,
            );
        }
        ui.separator();

        self.entries
            .show(ui, frame, url, &self.ladders[self.selected_ladder]);
    }

    fn show_ladders_combo_box(&mut self, ui: &mut Ui) -> bool {
        ComboBox::new("ladders", "Record Tables")
            .selected_text(&self.ladders[self.selected_ladder].name)
            .width(400.0)
            .show_ui(ui, |ui| {
                self.ladders.iter().enumerate().any(|(index, ladder)| {
                    ui.selectable_value(&mut self.selected_ladder, index, &ladder.name)
                        .changed()
                })
            })
            .inner
            .unwrap_or(false)
    }
}

enum Entries {
    Loading(Option<JoinHandle<Self>>),
    Loaded(LoadedEntries),
    LoadError(String),
}

impl Entries {
    fn show(&mut self, ui: &mut Ui, frame: &Frame, url: Url, selected_ladder: &Ladder) {
        match self {
            Entries::Loading(join_handle) => {
                if join_handle.as_ref().unwrap().is_finished() {
                    let join_handle = join_handle.take().unwrap();
                    *self = join_handle.join().unwrap();
                    ui.ctx().request_repaint_of(ViewportId::ROOT);
                }

                ui.add_space(20.0);
                ui.label("loading table entries...");
                ui.add_space(40.0);
                ui.label(WidgetText::from("⏳").color(Color32::YELLOW));
                ui.add_space(20.0);
            }
            Entries::Loaded(entries) => {
                let search = ui
                    .horizontal(|ui| {
                        let mut search = TextEdit::singleline(&mut entries.search_player)
                            .desired_width(400.0)
                            .hint_text("search for Player")
                            .show(ui)
                            .response
                            .lost_focus()
                            && ui.input(|i| i.key_pressed(Key::Enter));
                        search |= ui.button("Search").clicked();
                        search
                    })
                    .inner;

                let mut change_page = None;
                ui.horizontal(|ui| {
                    ui.label("Page:");
                    ui.add_enabled_ui(entries.page > 1, |ui| {
                        if ui.button("⏴").clicked() {
                            change_page = Some(entries.page - 1);
                        }
                    });
                    if NumberEdit::new(&mut entries.entered_page, "page edit")
                        .clamp_min(1)
                        .clamp_max(entries.page_count)
                        .desired_text_edit_width(40.0)
                        .show(ui)
                        .lost_focus()
                        && entries.page != entries.entered_page
                    {
                        change_page = Some(entries.entered_page);
                    }
                    ui.add_enabled_ui(entries.page < entries.page_count, |ui| {
                        if ui.button("⏵").clicked() {
                            change_page = Some(entries.page + 1);
                        }
                    });

                    ui.add_space(20.0);
                    ui.checkbox(&mut entries.show_full_data, "Show full data");
                });
                entries.show(ui, frame, &url);
                if search {
                    *self = Self::begin_load_ladder_entries(
                        ui.ctx().clone(),
                        url.clone(),
                        selected_ladder.clone(),
                        1,
                        entries.search_player.clone(),
                        entries.show_full_data,
                    );
                } else if let Some(change_page) = change_page {
                    *self = Self::begin_load_ladder_entries(
                        ui.ctx().clone(),
                        url,
                        selected_ladder.clone(),
                        change_page,
                        entries.search_player.clone(),
                        entries.show_full_data,
                    );
                }
            }
            Entries::LoadError(err) => {
                ui.label(&*err);
            }
        }
    }

    fn begin_load_ladder_entries(
        ctx: Context,
        url: Url,
        ladder: Ladder,
        page: i32,
        search_player: String,
        show_full_data: bool,
    ) -> Entries {
        let join_handle = spawn_request(move || {
            Self::load_ladder_entries(ctx, url, ladder, page, search_player, show_full_data)
        });
        Entries::Loading(Some(join_handle))
    }

    fn load_ladder_entries(
        ctx: Context,
        url: Url,
        ladder: Ladder,
        page: i32,
        search_player: String,
        show_full_data: bool,
    ) -> Entries {
        let state = match Self::do_load_ladder_entries(url, ladder, page, &search_player) {
            Ok(entries) => Entries::Loaded(LoadedEntries::new(
                page,
                entries,
                search_player,
                show_full_data,
            )),
            Err(err) => Entries::LoadError(format!(
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
        search_player: &str,
    ) -> Result<LadderEntriesModel, RequestError> {
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

        if !search_player.is_empty() {
            query.push(("player__icontains", search_player)); // i for case insensitive
        }
        let response = client.get(url).query(&query).send()?;
        if !response.status().is_success() {
            return Err(RequestError::from(response));
        }
        let ladder_entries = response.json::<LadderEntriesModel>()?;
        Ok(ladder_entries)
    }
}

struct LoadedEntries {
    page: i32,
    entered_page: i32,
    selected_row: Option<usize>,
    page_count: i32,
    data_headers: Vec<String>,
    cherry_pick_indices: Vec<usize>,
    entries: Vec<LadderEntry>,
    download_log_state: DownloadLogState,
    search_player: String,
    show_full_data: bool,
}

impl LoadedEntries {
    fn new(
        page: i32,
        model: LadderEntriesModel,
        search_player: String,
        show_full_data: bool,
    ) -> Self {
        let mut formatter = NumberFormatter::new();
        let data_headers: Vec<_> = model
            .results
            .first()
            .map(|e| {
                ["Rank".to_owned(), "Player".to_owned()]
                    .into_iter()
                    .chain(e.data.keys().cloned().map(|h| h.replace('_', " ")))
                    .chain(std::iter::once("Date".to_owned()))
                    .collect()
            })
            .unwrap_or(Vec::new());
        let entries: Vec<_> = model
            .results
            .into_iter()
            .map(|e| LadderEntry::new(e, &mut formatter))
            .collect();
        let cherry_pick_indices: Vec<_> = CHERRY_DATA_PICKS
            .iter()
            .filter_map(|d| data_headers.iter().position(|k| k == d))
            .collect();
        Self {
            page_count: model.count / PAGE_SIZE + if model.count % PAGE_SIZE > 0 { 1 } else { 0 },
            page,
            entered_page: page,
            data_headers,
            cherry_pick_indices,
            entries,
            selected_row: None,
            download_log_state: DownloadLogState::Idle,
            search_player,
            show_full_data,
        }
    }

    fn show(&mut self, ui: &mut Ui, frame: &Frame, url: &Url) {
        if self.entries.len() == 0 {
            ui.label("no entries");
            return;
        }

        ScrollArea::horizontal().show(ui, |ui| {
            Table::new(ui)
                .header(15.0, |r| {
                    let headers = if self.show_full_data {
                        Either::Left(self.data_headers.iter())
                    } else {
                        Either::Right(
                            self.cherry_pick_indices
                                .iter()
                                .copied()
                                .map(|i| &self.data_headers[i]),
                        )
                    };
                    for header in headers {
                        r.cell(|ui| {
                            ui.label(&*header);
                        });
                    }
                    r.cell(|ui| {
                        ui.label("📥");
                    })
                    .on_hover_text("download log");
                })
                .body(25.0, |b| {
                    for (index, entry) in self.entries.iter().enumerate() {
                        if b.selectable_row(self.selected_row == Some(index), |r| {
                            let data = if self.show_full_data {
                                Either::Left(entry.data.iter())
                            } else {
                                Either::Right(
                                    self.cherry_pick_indices
                                        .iter()
                                        .copied()
                                        .map(|i| &entry.data[i]),
                                )
                            };
                            for data in data {
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

                            self.download_log_state.show_download_button(
                                r,
                                frame,
                                url,
                                entry.combatlog_id,
                            );
                        })
                        .clicked()
                        {
                            if self.selected_row == Some(index) {
                                self.selected_row = None
                            } else {
                                self.selected_row = Some(index)
                            }
                        }
                    }
                });
        });

        self.download_log_state.show_download(ui);
    }
}

enum DownloadLogState {
    Idle,
    Downloading(String, Option<JoinHandle<Self>>),
    DownloadFailed(String),
}

impl DownloadLogState {
    fn is_idle(&self) -> bool {
        match self {
            DownloadLogState::Idle => true,
            _ => false,
        }
    }

    fn show_download_button(&mut self, row: &mut TableRow, frame: &Frame, url: &Url, log_id: i32) {
        if row
            .selectable_cell(false, |ui| {
                ui.add_enabled_ui(self.is_idle(), |ui| {
                    ui.label("📥");
                });
            })
            .on_hover_text("download log")
            .clicked()
        {
            if let Some(file) = rfd::FileDialog::new()
                .set_parent(frame)
                .set_title("Download combatlog File")
                .add_filter("combatlog", &["log"])
                .save_file()
            {
                *self = Self::begin_download_log(url.clone(), file, log_id);
            }
        }
    }

    fn show_download(&mut self, ui: &Ui) {
        match self {
            DownloadLogState::Idle => (),
            DownloadLogState::Downloading(message, join_handle) => {
                Window::new("Download log")
                    .auto_sized()
                    .constrain(true)
                    .collapsible(false)
                    .show(ui.ctx(), |ui| {
                        ui.add_space(20.0);
                        ui.label(&*message);
                        ui.add_space(40.0);
                        ui.label(WidgetText::from("⏳").color(Color32::YELLOW));
                        ui.add_space(20.0);
                    });
                if join_handle.as_ref().unwrap().is_finished() {
                    *self = join_handle.take().unwrap().join().unwrap();
                    ui.ctx().request_repaint_of(ViewportId::ROOT);
                }
            }
            DownloadLogState::DownloadFailed(error) => {
                let mut open = true;
                Window::new("Download log failed")
                    .auto_sized()
                    .constrain(true)
                    .collapsible(false)
                    .open(&mut open)
                    .show(ui.ctx(), |ui| {
                        ui.label(&*error);
                    });

                if !open {
                    *self = DownloadLogState::Idle;
                }
            }
        }
    }

    fn begin_download_log(url: Url, path: PathBuf, log_id: i32) -> DownloadLogState {
        DownloadLogState::Downloading(
            format!("downloading log to {:?}...", path),
            Some(spawn_request(move || Self::download_log(url, path, log_id))),
        )
    }

    fn download_log(url: Url, path: PathBuf, log_id: i32) -> DownloadLogState {
        match Self::do_download_log(url, path, log_id) {
            Ok(_) => DownloadLogState::Idle,
            Err(err) => DownloadLogState::DownloadFailed(
                err.action_error("Failed to download log.").to_string(),
            ),
        }
    }

    fn do_download_log(mut url: Url, path: PathBuf, log_id: i32) -> Result<(), RequestError> {
        let client = ClientBuilder::new().build().unwrap();
        url.set_path(&format!("/combatlog/{}/download/", log_id));
        let mut response = client.get(url).send()?;
        if !response.status().is_success() {
            return Err(RequestError::from(response));
        }

        let mut data = Vec::new();
        response.copy_to(&mut data)?;

        let file = File::create(path)?;
        GzDecoder::new(file).write_all(&data)?;

        Ok(())
    }
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
    combatlog: i32,
    data: serde_json::Map<String, serde_json::Value>,
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

struct LadderEntry {
    data: Vec<DataValue>,
    combatlog_id: i32,
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
            combatlog_id: model.combatlog,
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
