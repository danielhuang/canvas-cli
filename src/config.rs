use color_eyre::eyre::Result;
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub canvas_url: String,
    pub token: String,
    #[serde(default)]
    pub gradescope_cookie: Option<String>,
    #[serde(default)]
    pub hide_overdue_after_days: Option<i64>,
    #[serde(default)]
    pub hide_overdue_without_submission: bool,
    #[serde(default)]
    pub exclude: Vec<Exclusion>,
    #[serde(default)]
    pub include: Vec<Inclusion>,
    #[serde(default)]
    pub hide_locked: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Exclusion {
    ByClassId { class_id: i64 },
    ByAssignmentId { assignment_id: i64 },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Inclusion {
    ByAssignmentId { assignment_id: i64 },
}

pub fn read_config() -> Result<Config> {
    let config = read_to_string(config_path())?;
    Ok(toml::from_str(&config)?)
}

pub fn config_path() -> std::path::PathBuf {
    home_dir().unwrap().join(".canvas.toml")
}
