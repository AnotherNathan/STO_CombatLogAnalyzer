#![allow(non_snake_case)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::backtrace::Backtrace;

use app::logging;
use eframe::{epaint::vec2, IconData};

mod analyzer;
mod app;
mod custom_widgets;
mod helpers;

fn main() {
    std::panic::set_hook(Box::new(|i| {
        log::error!("{}", i);
        let backtrace = Backtrace::capture();
        log::error!("backtrace:");
        log::error!("{}", backtrace);
    }));

    logging::initialize();
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(vec2(1280.0, 720.0)),
        min_window_size: Some(vec2(480.0, 270.0)),
        icon_data: Some(icon_data()),
        ..Default::default()
    };

    let res = eframe::run_native(
        &format!("STO_CombatLogAnalyzer V{}", env!("CARGO_PKG_VERSION")),
        native_options,
        Box::new(|cc| Box::new(app::App::new(cc))),
    );

    if let Err(err) = res {
        log::error!("eframe crashed: {}", err);
    }
}

fn icon_data() -> IconData {
    const ICON: &[u8] = include_bytes!("../icon/icon.png");
    let decoder = png::Decoder::new(ICON);
    let mut reader = decoder.read_info().unwrap();
    let mut data = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut data).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    IconData {
        rgba: data,
        width: info.width,
        height: info.height,
    }
}
