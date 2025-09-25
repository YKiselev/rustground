use std::io::{Read, Write};

use log::warn;
use rg_common::files::Files;
use toml::Table;

use crate::AppFiles;

pub fn load_config(name: &str, files: &AppFiles) -> Option<Table> {
    let mut cfg = files.read(name)?;
    let mut tmp = String::new();
    let _ = cfg
        .read_to_string(&mut tmp)
        .expect("Unable to read from file!");
    toml::from_str::<Table>(&tmp).ok()
}

pub fn save_config(name: &str, files: &AppFiles, value: String) {
    if let Some(mut file) = files.write(name) {
        match write!(file, "{}", value) {
            Ok(_) => {
                file.flush().unwrap();
            },
            Err(e) => {
                warn!("Unable to save config: {:?}", e)
            },
        }
    }
}
