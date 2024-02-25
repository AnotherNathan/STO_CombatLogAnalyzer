use std::{io::Write, thread::JoinHandle, time::Duration};

use eframe::egui::*;
use reqwest::{
    blocking::{
        multipart::{Form, Part},
        ClientBuilder,
    },
    Url,
};
use serde::Deserialize;

use crate::{
    analyzer::{settings::AnalysisSettings, Combat},
    custom_widgets::table::Table,
};

use super::common::{spawn_request, RequestError};

#[derive(Default)]
pub struct Upload {
    state: UploadState,
}

const UPLOAD_TOOLTIP: &str = "Uploads the current combat to the records (powered by OSCR). Note that the uploaded values may vary compared to the values displayed here, since the calculations may be done differently.";

impl Upload {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        combat: Option<&Combat>,
        settings: &AnalysisSettings,
        url: &str,
    ) {
        ui.add_enabled_ui(self.state.is_idle() && combat.is_some(), |ui| {
            if ui
                .button("Upload ☁")
                .on_hover_text(UPLOAD_TOOLTIP)
                .clicked()
            {
                self.state = self.begin_upload(ui.ctx().clone(), combat.unwrap(), settings, url);
            };
        });
        match &mut self.state {
            UploadState::Idle => (),
            UploadState::Uploading(join_handle) => {
                if join_handle.as_ref().unwrap().is_finished() {
                    self.state = join_handle.take().unwrap().join().unwrap();
                    ui.ctx().request_repaint_of(ViewportId::ROOT);
                }

                Self::window(ui, true, |ui| {
                    ui.with_layout(Layout::top_down(Align::Center), |ui| {
                        ui.add_space(20.0);
                        ui.label("uploading...");
                        ui.add_space(40.0);
                        ui.label(WidgetText::from("⏳").color(Color32::YELLOW));
                        ui.add_space(20.0);
                    });
                });
            }
            UploadState::UploadComplete(result) => {
                if let Some(true) = Self::window(ui, false, |ui| {
                    Table::new(ui)
                        .header(15.0, |r| {
                            r.cell(|ui| {
                                ui.label("Name");
                            });
                            r.cell(|ui| {
                                ui.label("Updated");
                            });
                            r.cell(|ui| {
                                ui.label("Details");
                            });
                        })
                        .body(25.0, |b| {
                            for result in result.iter() {
                                b.row(|r| {
                                    r.cell(|ui| {
                                        ui.label(&result.name);
                                    });
                                    r.cell_with_layout(
                                        Layout::top_down(Align::Center)
                                            .with_cross_align(Align::Center),
                                        |ui| {
                                            let text = match result.updated {
                                                true => WidgetText::from("✔").color(Color32::GREEN),
                                                false => WidgetText::from("✖").color(Color32::RED),
                                            };
                                            ui.label(text);
                                        },
                                    );
                                    r.cell(|ui| {
                                        ui.label(&result.detail);
                                    });
                                });
                            }
                        });
                    ui.add_space(40.0);
                    if ui.button("Close").clicked() {
                        true
                    } else {
                        false
                    }
                }) {
                    self.state = UploadState::Idle;
                }
            }
            UploadState::UploadError(error) => {
                if let Some(true) = Self::window(ui, false, |ui| {
                    ui.label(&*error);
                    ui.add_space(40.0);
                    if ui.button("Close").clicked() {
                        true
                    } else {
                        false
                    }
                }) {
                    self.state = UploadState::Idle;
                }
            }
        }
    }

    fn window<R>(ui: &Ui, constrain: bool, add_contents: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let mut window = Window::new("Upload")
            .collapsible(false)
            .auto_sized()
            .constrain(true);

        if constrain {
            window = window.max_size([360.0, 480.0]);
        }

        window
            .show(ui.ctx(), add_contents)
            .map(|r| r.inner)
            .flatten()
    }

    fn begin_upload(
        &self,
        ctx: Context,
        combat: &Combat,
        settings: &AnalysisSettings,
        url: &str,
    ) -> UploadState {
        let combat_data = combat.read_log_combat_data(settings.combatlog_file());
        let combat_data = match combat_data {
            Some(d) => d,
            None => return UploadState::Idle,
        };
        let url = match Url::parse(url) {
            Ok(u) => u,
            Err(_) => {
                return UploadState::UploadError("the provided upload URL is invalid".into());
            }
        };
        let combat_name = combat.name();
        let join_handle = spawn_request(move || Self::upload(ctx, url, combat_data, combat_name));
        UploadState::Uploading(Some(join_handle))
    }

    fn upload(ctx: Context, url: Url, combat_data: Vec<u8>, combat_name: String) -> UploadState {
        let state = match Self::do_upload(url, combat_data, combat_name) {
            Ok(r) => UploadState::UploadComplete(r),
            Err(e) => UploadState::UploadError(format!(
                "{}",
                e.action_error("Failed to upload combat log.")
            )),
        };
        ctx.request_repaint_after_for(Duration::from_millis(10), ViewportId::ROOT);
        state
    }

    fn do_upload(
        mut url: Url,
        combat_data: Vec<u8>,
        combat_name: String,
    ) -> Result<Vec<UploadResponse>, RequestError> {
        let mut data = Vec::new();
        let mut encoder = flate2::GzBuilder::new().write(&mut data, flate2::Compression::best());
        encoder.write_all(combat_data.as_slice()).unwrap();
        encoder.finish().unwrap();
        let client = ClientBuilder::new().build().unwrap();
        url.set_path("/combatlog/upload/");
        let form = Form::new().part("file", Part::bytes(data).file_name(combat_name));
        let response = client.post(url).multipart(form).send()?;
        if !response.status().is_success() {
            return Err(RequestError::from(response));
        }

        let response = response.json::<Vec<UploadResponse>>()?;
        Ok(response)
    }
}

#[derive(Default)]
enum UploadState {
    #[default]
    Idle,
    Uploading(Option<JoinHandle<Self>>),
    UploadComplete(Vec<UploadResponse>),
    UploadError(String),
}

impl UploadState {
    fn is_idle(&self) -> bool {
        match self {
            UploadState::Idle => true,
            _ => false,
        }
    }
}

#[derive(Deserialize)]
struct UploadResponse {
    name: String,
    updated: bool,
    detail: String,
}
