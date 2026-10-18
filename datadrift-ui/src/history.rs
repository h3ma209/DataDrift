//! Query execution history (DataGrip guide §7.6 — local per-machine).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub at: String,
    pub vendor: String,
    pub source: String,
    pub sql: String,
    pub ok: bool,
    pub message: String,
}

fn path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".datadrift").join("query_history.json")
}

pub fn load() -> Vec<HistoryEntry> {
    let Ok(data) = fs::read_to_string(path()) else {
        return vec![];
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn push(entry: HistoryEntry) {
    let mut all = load();
    all.insert(0, entry);
    all.truncate(200);
    let _ = fs::create_dir_all(path().parent().unwrap());
    if let Ok(json) = serde_json::to_string_pretty(&all) {
        let _ = fs::write(path(), json);
    }
}
