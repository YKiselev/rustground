use std::io::Read;


use rg_common::files;
use rg_common::files::Files;
use toml::Table;


pub fn load_config(name: &str, files: &mut files::AppFiles) -> Option<Table> {
    let mut cfg = files.open(name).expect("Unable to load config!");
    let mut tmp = String::new();
    let _ = cfg
        .read_to_string(&mut tmp)
        .expect("Unable to read from file!");
    toml::from_str::<Table>(&tmp).ok()
}
