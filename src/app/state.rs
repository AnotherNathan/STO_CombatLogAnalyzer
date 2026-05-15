use eframe::egui::Context;

use crate::app::overlay::Overlay;

use super::{analysis_handling::AnalysisHandler, settings::Settings};

pub struct AppState {
    pub settings: Settings,
    pub analysis_handler: AnalysisHandler,
    pub overlay: Overlay,
}

impl AppState {
    pub fn new(ctx: &Context) -> Self {
        let settings = Settings::load_or_default();
        let analysis_handler = AnalysisHandler::new(
            settings.analysis.clone(),
            ctx.clone(),
            settings.auto_refresh.interval_seconds,
            settings.auto_refresh.enable,
        );

        Self {
            overlay: Overlay::new(&analysis_handler, &settings),
            settings,
            analysis_handler,
        }
    }
}
