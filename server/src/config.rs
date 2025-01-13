use serde::Deserialize;
use std::{collections::HashMap, fs};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigParsingError {
    #[error("failed to parse TOML file")]
    ParseError(#[from] toml::de::Error),
    #[error("failed to read TOML file")]
    ReadError(#[from] std::io::Error),
}

#[derive(Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub peer: HashMap<String, Peer>,
}

#[derive(Deserialize, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub folder: String,
    pub private_key: String,
}

#[derive(Deserialize, Clone)]
pub struct Peer {
    pub username: String,
}

pub fn get_config(path: &str) -> Result<Config, ConfigParsingError> {
    let config_file = fs::read_to_string(path)?;
    let config = toml::from_str(&config_file)?;

    Ok(config)
}
