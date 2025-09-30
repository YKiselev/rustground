
use toml::Table;

use crate::Files;

pub fn load_config(name: &str, files: &Files) -> Option<Table> {
    let cfg = files.read_file(name)?;
    toml::from_str::<Table>(&cfg).ok()
}

pub fn save_config(name: &str, files: &Files, value: String) {
    files.write_file(name, value);
}
