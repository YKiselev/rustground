use std::io::Read;

use serde::Deserialize;

use common::files;

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub server: ServerConfig,
    pub client: ClientConfig,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ServerConfig {
    pub address: String,
    pub key_bits: usize,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClientConfig {}

impl Config {
    pub(crate) fn load(name: &str, files: &mut files::Files) -> Self {
        let mut cfg = files.open(name).expect("Unable to load config!");
        let mut tmp = String::new();
        let read = cfg.read_to_string(&mut tmp).expect("Unable to read from file!");
        toml::from_str(&tmp).expect("Unable to deserialize!")
    }
}