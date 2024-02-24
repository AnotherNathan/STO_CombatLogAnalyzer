use std::{fmt::Display, io::Write, mem::replace, thread::JoinHandle};

use eframe::egui::*;
use reqwest::{
    blocking::{
        multipart::{Form, Part},
        ClientBuilder, Response,
    },
    Error, StatusCode, Url,
};
use serde::Deserialize;

use crate::{
    analyzer::{settings::AnalysisSettings, Combat},
    custom_widgets::table::Table,
};

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
                self.state = self.begin_upload(combat.unwrap(), settings, url);
            };
        });
        match &self.state {
            UploadState::Idle => (),
            UploadState::Uploading(join_handle) => {
                if join_handle.is_finished() {
                    let UploadState::Uploading(join_handle) =
                        replace(&mut self.state, UploadState::Idle)
                    else {
                        panic!()
                    };
                    self.state = join_handle.join().unwrap();
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
                    ui.label(error);
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

    fn begin_upload(&self, combat: &Combat, settings: &AnalysisSettings, url: &str) -> UploadState {
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
        let join_handle = std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(move || Self::upload(url, combat_data, combat_name))
            .unwrap();
        UploadState::Uploading(join_handle)
    }

    fn upload(url: Url, combat_data: Vec<u8>, combat_name: String) -> UploadState {
        match Self::do_upload(url, combat_data, combat_name) {
            Ok(r) => UploadState::UploadComplete(r),
            Err(e) => UploadState::UploadError(format!("{}", e)),
        }
    }

    fn do_upload(
        mut url: Url,
        combat_data: Vec<u8>,
        combat_name: String,
    ) -> Result<Vec<UploadResponse>, UploadError> {
        let mut data = Vec::new();
        let mut encoder = flate2::GzBuilder::new().write(&mut data, flate2::Compression::best());
        encoder.write_all(combat_data.as_slice()).unwrap();
        encoder.finish().unwrap();
        let client = ClientBuilder::new().build().unwrap();
        url.set_path("/combatlog/upload/");
        let form = Form::new().part("file", Part::bytes(data).file_name(combat_name));
        let response = client.post(url).multipart(form).send()?;
        if !response.status().is_success() {
            return Err(UploadError::from(response));
        }

        let response = response.json::<Vec<UploadResponse>>()?;
        Ok(response)
    }
}

#[derive(Default)]
enum UploadState {
    #[default]
    Idle,
    Uploading(JoinHandle<Self>),
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

enum UploadError {
    Status(StatusCode, Option<String>),
    Err(Error),
}

impl UploadError {
    fn base_msg(f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to upload combat log.")
    }

    fn status_code(f: &mut std::fmt::Formatter<'_>, status: StatusCode) -> std::fmt::Result {
        write!(f, "\n\nStatus Code: {}", status)
    }

    fn details_or_status_and_error(
        f: &mut std::fmt::Formatter<'_>,
        status: StatusCode,
        error: &Option<String>,
    ) -> std::fmt::Result {
        match error
            .as_ref()
            .map(|e| serde_json::from_str::<ServerError>(e).ok())
            .flatten()
        {
            Some(error) => write!(f, "\n\nDetails: {}", error.detail)?,
            None => {
                Self::status_code(f, status)?;
                Self::error(f, error)?;
            }
        }

        Ok(())
    }

    fn error(f: &mut std::fmt::Formatter<'_>, error: &Option<String>) -> std::fmt::Result {
        if let Some(error) = error.as_ref() {
            write!(f, "\n\nError: {}", error)?;
        }

        Ok(())
    }
}

impl Display for UploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Self::base_msg(f)?;
        match self {
            UploadError::Status(status, error) => {
                if *status == StatusCode::INTERNAL_SERVER_ERROR {
                    Self::details_or_status_and_error(f, *status, error)?;
                } else {
                    Self::status_code(f, *status)?;
                    Self::error(f, error)?;
                }
            }
            UploadError::Err(err) => write!(f, "Failed to upload combat log.\n\nError: {}", err)?,
        }

        Ok(())
    }
}

impl From<Error> for UploadError {
    fn from(value: Error) -> Self {
        Self::Err(value)
    }
}

impl From<Response> for UploadError {
    fn from(value: Response) -> Self {
        Self::Status(value.status(), value.text().ok())
    }
}

#[derive(Deserialize)]
struct UploadResponse {
    name: String,
    updated: bool,
    detail: String,
}

#[derive(Deserialize)]
struct ServerError {
    detail: String,
}
