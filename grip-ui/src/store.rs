use crate::model::DataSource;
use std::fs;
use std::path::PathBuf;

pub fn config_path() -> PathBuf {
    dirs_path().join("datasources.json")
}

fn dirs_path() -> PathBuf {
    if let Ok(p) = std::env::var("GRIP_CONFIG") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let p = PathBuf::from(home).join(".grip");
    let _ = fs::create_dir_all(&p);
    p
}

pub fn load() -> Vec<DataSource> {
    let path = config_path();
    let Ok(data) = fs::read_to_string(&path) else {
        return vec![DataSource::new(1)];
    };
    serde_json::from_str(&data).unwrap_or_else(|_| vec![DataSource::new(1)])
}

pub fn save(sources: &[DataSource]) -> Result<(), String> {
    let path = config_path();
    let mut to_save: Vec<DataSource> = sources.to_vec();
    for ds in &mut to_save {
        if !ds.remember_password {
            ds.password.clear();
        }
    }
    let json = serde_json::to_string_pretty(&to_save).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}
