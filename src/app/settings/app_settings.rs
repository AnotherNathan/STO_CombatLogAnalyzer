use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::analyzer::settings::AnalysisSettings;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Settings {
    pub analysis: AnalysisSettings,
    pub auto_refresh: AutoRefresh,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutoRefresh {
    pub enable: bool,
    pub interval_seconds: f64,
}

impl Settings {
    fn file_path() -> Option<PathBuf> {
        let mut path = std::env::current_exe().ok()?;
        path.pop();
        path.push("settings.json");
        Some(path)
    }

    pub fn load_or_default() -> Self {
        Self::file_path()
            .and_then(|f| std::fs::read_to_string(&f).ok())
            .map(|d| serde_json::from_str(&d).ok())
            .flatten()
            .unwrap_or_else(|| Self::default())
    }

    pub fn save(&self) {
        let file_path = match Self::file_path() {
            Some(p) => p,
            None => {
                return;
            }
        };
        let data = match serde_json::to_string_pretty(self) {
            Ok(d) => d,
            Err(_) => {
                return;
            }
        };

        let _ = std::fs::write(&file_path, data);
    }
}

impl AutoRefresh {
    pub fn interval_seconds(&self) -> Option<f64> {
        if self.enable {
            Some(self.interval_seconds)
        } else {
            None
        }
    }
}

impl Default for AutoRefresh {
    fn default() -> Self {
        Self {
            enable: false,
            interval_seconds: 4.0,
        }
    }
}
