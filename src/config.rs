use color_eyre::eyre::Result;
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub canvas_url: String,
    pub token: String,
    #[serde(default)]
    pub overdue_offset: Option<i64>,
    #[serde(default)]
    pub exclude: Vec<Exclusion>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Exclusion {
    ByClassId { class_id: i64 },
    ByAssignmentId { assignment_id: i64 },
}

pub fn read_config() -> Result<Config> {
    let config = read_to_string(home_dir().unwrap().join(".canvas.toml"))?;
    Ok(toml::from_str(&config)?)
}
