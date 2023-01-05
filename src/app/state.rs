use eframe::egui::Context;

use super::{analysis_handling::AnalysisHandler, settings::Settings};

pub struct AppState {
    pub settings: Settings,
    pub analysis_handler: AnalysisHandler,
}

impl AppState {
    pub fn new(ctx: &Context) -> Self {
        let settings = Settings::load_or_default();
        let analysis_handler = AnalysisHandler::new(
            settings.analysis.clone(),
            ctx.clone(),
            settings.auto_refresh.interval_seconds(),
        );

        Self {
            settings,
            analysis_handler,
        }
    }
}
