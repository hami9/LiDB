use crate::theme::ThemeMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub refresh_rate_ms: u64,
    pub theme_mode: ThemeMode,
    pub paused: bool,
    pub show_simulated_fixtures: bool,
    pub compact_view: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            refresh_rate_ms: 500,
            theme_mode: ThemeMode::Dark,
            paused: false,
            show_simulated_fixtures: true,
            compact_view: false,
        }
    }
}
