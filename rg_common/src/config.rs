use std::io::Read;

use toml::Table;

use crate::{Files, LoaderError};

pub fn read_config<R>(reader: &mut R) -> Result<Table, LoaderError>
where
    R: Read,
{
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    toml::from_str::<Table>(&buf).map_err(|e| LoaderError::Custom(e.to_string()))
}

// pub fn load_config(name: &str, files: &Files) -> Option<Table> {
//     let cfg = files.read_file(name)?;
//     toml::from_str::<Table>(&cfg).ok()
// }

pub fn save_config(name: &str, files: &Files, value: String) {
    files.write_file(name, value);
}
