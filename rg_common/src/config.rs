use std::io::Read;

use log::info;
use serde::{Deserialize, Serialize};

use rg_common::files;
use rg_common::files::Files;
use rg_macros::VarBag;
use toml::Table;

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
        let _ = cfg
            .read_to_string(&mut tmp)
            .expect("Unable to read from file!");
        let table = toml::from_str::<Table>(&tmp).expect("Unable to deserialize!");

        for v in table.iter() {
            info!("{:?}={:?}", v.0, v.1);
        }

        Self {
            server: ServerConfig {
                address: "localhost".to_owned(),
                bound_to: None,
                key_bits: 512,
                password: None,
            },
            client: ClientConfig {},
        }
    }
}
