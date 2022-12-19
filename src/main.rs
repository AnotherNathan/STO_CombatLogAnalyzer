#![windows_subsystem = "windows"]
#![allow(non_snake_case)]

mod analyzer;
mod app;
mod custom_widgets;
mod helpers;
mod parser;

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "STO_CombatlogAnalyzer",
        native_options,
        Box::new(|cc| Box::new(app::App::new(cc))),
    );
}
