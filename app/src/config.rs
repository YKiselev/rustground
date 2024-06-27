use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub server: ServerConfig,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ServerConfig {
    pub address: String,
    pub key_bits: usize,
    password: Option<String>,
}