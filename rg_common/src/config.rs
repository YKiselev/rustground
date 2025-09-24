use std::io::{Read, Write};

use log::{info, warn};
use rg_common::files::Files;
use toml::Table;

use crate::AppFiles;

pub fn load_config(name: &str, files: &mut AppFiles) -> Option<Table> {
    let mut cfg = files.open(name)?;
    let mut tmp = String::new();
    let _ = cfg
        .read_to_string(&mut tmp)
        .expect("Unable to read from file!");
    toml::from_str::<Table>(&tmp).ok()
}

pub fn save_config(name: &str, files: &mut AppFiles, value: String) {
    if let Some(mut file) = files.create(name) {
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
