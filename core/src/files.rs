use std::io::Error;
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use log::warn;
use crate::arguments::Arguments;

struct FileRoot {
    readonly: bool,
    path: PathBuf,
}

impl FileRoot {
    fn try_new(path: &str) -> Result<Self, Error> {
        let metadata = fs::metadata(path)?;
        let readonly = metadata.permissions().readonly();
        Ok(FileRoot {
            readonly,
            path: PathBuf::from(path),
        })
    }

    fn readonly(&self) -> bool {
        self.readonly
    }
}

pub struct Files {
    roots: Vec<FileRoot>,
}

impl Files {
    pub fn new(args: &Arguments) -> Self {
        Files {
            roots: args.base().iter().map(|v| {
                let r = FileRoot::try_new(v);
                if r.is_err() {
                    return r.inspect_err(|error| {
                        warn!("Failed to map \"{v}\": {error}");
                    });
                }
                r
            }).filter(Result::is_ok)
                .map(Result::unwrap)
                .collect()
        }
    }

    pub fn open(path: &str) -> File {
        unimplemented!()
    }
}
