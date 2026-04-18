use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    pub reset_offset: bool,
    pub button_scale: u32,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            reset_offset: true,
            button_scale: 100,
        }
    }
}

pub fn write_settings(file_path: &str, settings: &Settings) -> Result<()> {
    let json_string =
        serde_json::to_string_pretty(settings).context("Failed to serialize settings")?;
    fs::write(file_path, json_string).context("Failed to write settings file")?;
    Ok(())
}

pub fn read_settings(file_path: &str) -> Result<Settings> {
    let read_json_string = fs::read_to_string(file_path).context("Failed to read settings file")?;
    let settings =
        serde_json::from_str(&read_json_string).context("Failed to parse settings JSON")?;
    Ok(settings)
}
