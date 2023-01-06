use std::sync::Arc;

use eframe::egui::*;

use self::{analysis_handling::AnalysisInfo, main_tabs::*, settings::*, state::AppState};

mod analysis_handling;
pub mod logging;
mod main_tabs;
mod settings;
mod state;

pub struct App {
    settings_window: SettingsWindow,
    combats: Vec<String>,
    selected_combat: Option<usize>,
    main_tabs: MainTabs,
    state: AppState,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let mut style = Style::clone(&cc.egui_ctx.style());
        style.override_font_id = Some(FontId::monospace(12.0));
        cc.egui_ctx.set_style(Arc::new(style));
        let state = AppState::new(&cc.egui_ctx);
        let settings_window =
            SettingsWindow::new(&cc.egui_ctx, cc.integration_info.native_pixels_per_point);
        Self {
            settings_window,
            combats: Default::default(),
            selected_combat: None,
            main_tabs: MainTabs::empty(),
            state,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        self.handle_analysis_infos();

        CentralPanel::default().show(ctx, |ui| {
            self.settings_window.show(&mut self.state, ui, frame);

            ui.horizontal(|ui| {
                ComboBox::new("combat list", "Combats")
                    .width(400.0)
                    .selected_text(self.main_tabs.identifier.as_str())
                    .show_ui(ui, |ui| {
                        for (i, combat) in self.combats.iter().enumerate().rev() {
                            if ui
                                .selectable_value(
                                    &mut self.selected_combat,
                                    Some(i),
                                    combat.as_str(),
                                )
                                .changed()
                            {
                                if let Some(combat_index) = self.selected_combat {
                                    self.state.analysis_handler.get_combat(combat_index);
                                }
                            }
                        }
                    });

                if ui.button("Refresh now").clicked() {
                    self.state.analysis_handler.refresh();
                }

                if self.state.analysis_handler.is_busy() {
                    ui.label("⏳ Working..");
                }
            });

            self.main_tabs.show(ui);
        });
    }
}

impl App {
    fn handle_analysis_infos(&mut self) {
        for info in self.state.analysis_handler.check_for_info() {
            match info {
                AnalysisInfo::Combat(combat) => self.main_tabs.update(&combat),
                AnalysisInfo::Refreshed {
                    latest_combat,
                    combats,
                } => {
                    self.main_tabs.update(&latest_combat);
                    self.combats = combats;
                    self.selected_combat = Some(self.combats.len());
                }
            }
        }
    }
}
