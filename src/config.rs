use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub canvas_url: String,
    pub token: String,
}

pub fn read_config() -> Config {
    let config = read_to_string(home_dir().unwrap().join(".canvas.toml")).unwrap();
    toml::from_str(&config).unwrap()
}
