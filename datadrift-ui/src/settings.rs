//! UI preferences (Spotify-like Settings).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct UiSettings {
    /// Prefer less motion.
    #[serde(default)]
    pub reduce_motion: bool,
    /// Compact row spacing in grids / library.
    #[serde(default)]
    pub compact: bool,
    /// Show ⌘⏎ hint under console.
    #[serde(default = "default_true")]
    pub show_shortcuts: bool,
    /// Soft green glow intensity 0–2.
    #[serde(default = "default_glow")]
    pub glow: u8,
}

fn default_true() -> bool {
    true
}
fn default_glow() -> u8 {
    1
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            reduce_motion: false,
            compact: false,
            show_shortcuts: true,
            glow: 1,
        }
    }
}

fn path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".datadrift").join("ui_settings.json")
}

pub fn load() -> UiSettings {
    let Ok(data) = fs::read_to_string(path()) else {
        return UiSettings::default();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save(s: &UiSettings) -> Result<(), String> {
    let _ = fs::create_dir_all(path().parent().unwrap());
    let json = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    fs::write(path(), json).map_err(|e| e.to_string())
}
