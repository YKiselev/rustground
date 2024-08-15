use std::io::Read;

use serde::{Deserialize, Serialize};

use rg_common::files;
use rg_common::files::Files;
use rg_macros::VarBag;

#[derive(Debug, Serialize, Deserialize, VarBag)]
pub struct Config {
    pub server: ServerConfig,
    pub client: ClientConfig,
}

#[derive(Debug, Serialize, Deserialize, VarBag)]
pub struct ServerConfig {
    pub address: String,
    #[serde(skip_serializing)]
    pub bound_to: Option<String>,
    pub key_bits: usize,
    pub password: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, VarBag)]
pub struct ClientConfig {}

impl Config {
    pub fn load(name: &str, files: &mut files::AppFiles) -> Self {
        let mut cfg = files.open(name).expect("Unable to load config!");
        let mut tmp = String::new();
        let read = cfg
            .read_to_string(&mut tmp)
            .expect("Unable to read from file!");
        toml::from_str(&tmp).expect("Unable to deserialize!")
    }
}
