
use toml::Table;

use crate::AppFiles;

pub fn load_config(name: &str, files: &AppFiles) -> Option<Table> {
    let cfg = files.read_file(name)?;
    toml::from_str::<Table>(&cfg).ok()
}

pub fn save_config(name: &str, files: &AppFiles, value: String) {
    files.write_file(name, value);
}
