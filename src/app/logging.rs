use std::{fs::OpenOptions, path::PathBuf};

use simplelog::{CombinedLogger, Config, SharedLogger, SimpleLogger, WriteLogger};

use super::settings::Settings;

pub fn initialize() {
    let settings = Settings::load_or_default();

    if !settings.debug.enable_log {
        return;
    }

    let mut loggers: Vec<Box<dyn SharedLogger>> = vec![SimpleLogger::new(
        settings.debug.log_level_filter,
        Config::default(),
    )];

    if let Some(file) = file_path()
        .map(|p| OpenOptions::new().create(true).append(true).open(&p).ok())
        .flatten()
    {
        loggers.push(WriteLogger::new(
            log::LevelFilter::Info,
            Config::default(),
            file,
        ));
    }

    CombinedLogger::init(loggers).unwrap();
}

fn file_path() -> Option<PathBuf> {
    let mut path = std::env::current_exe().ok()?;
    path.pop();
    path.push("STO_CombatLogAnalyzer.log");
    Some(path)
}
