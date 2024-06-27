use std::{env, fs};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Error;
use std::path::{Path, PathBuf};

use log::{debug, info, warn};

use crate::arguments::Arguments;

struct FileRoot {
    readonly: bool,
    path: PathBuf,
}

impl FileRoot {
    fn try_new(path: &Path) -> Result<Self, Error> {
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

    fn open(&mut self, path: &str) -> Option<File> {
        let mut buf = self.path.clone();
        buf.push(path);
        match File::open(buf.clone()) {
            Ok(file) => {
                Some(file)
            }
            Err(e) => {
                debug!("File not found: {:?}, {:?}", buf, e);
                None
            }
        }
    }
}

impl Display for FileRoot {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "FileRoot(read_only={}, path={})", self.readonly, self.path.display())
    }
}


pub struct Files {
    roots: Vec<FileRoot>,
}

impl Files {
    pub fn new(args: &Arguments) -> Self {
        let current_dir = if let Ok(dir) = env::current_dir() {
            dir
        } else {
            PathBuf::from(".")
        };
        let base_resources = current_dir.join("base/resources");
        let base = current_dir.join("base");
        let mut folders = vec![base_resources, base];
        if let Some(home) = dirs::home_dir() {
            folders.push(home);
        }
        folders.reverse();
        let roots = folders.iter().map(|path| {
            let r = FileRoot::try_new(path);
            if r.is_err() {
                r.inspect_err(|error| {
                    warn!("Failed to map \"{}\": {error}", path.display());
                })
            } else {
                r.inspect(|path| {
                    info!("Added path: {}", path);
                })
            }
        }).filter(Result::is_ok)
            .map(Result::unwrap)
            .collect();

        Files {
            roots
        }
    }

    pub fn open(&mut self, path: &str) -> Option<File> {
        self.roots.iter_mut().find_map(|r| r.open(path))
    }
}
