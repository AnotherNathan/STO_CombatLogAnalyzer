#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![cfg_attr(not(debug_assertions), allow(non_snake_case))]

use app::logging;
use eframe::epaint::{vec2, Vec2};

mod analyzer;
mod app;
mod custom_widgets;
mod helpers;

fn main() {
    logging::initialize();
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(vec2(1280.0, 720.0)),
        ..Default::default()
    };

    eframe::run_native(
        "STO_CombatLogAnalyzer",
        native_options,
        Box::new(|cc| Box::new(app::App::new(cc))),
    );
}
