use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Error;
use std::path::{Path, PathBuf};
use std::{env, fs};

use log::{debug, error, info, warn};

use crate::arguments::Arguments;

pub trait Files {
    fn open<S: AsRef<str>>(&mut self, path: S) -> Option<File>;
}

#[derive(Debug)]
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
            Ok(file) => Some(file),
            Err(e) => {
                debug!("File not found: {:?}, {:?}", buf, e);
                None
            }
        }
    }
}

impl Display for FileRoot {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FileRoot(read_only={}, path={})",
            self.readonly,
            self.path.display()
        )
    }
}

#[derive(Debug)]
pub struct AppFiles {
    roots: Vec<FileRoot>,
}

impl AppFiles {
    pub fn new(args: &Arguments) -> Self {
        let mut folders: Vec<PathBuf> = Vec::new();
        if let Some(home) = dirs::home_dir() {
            let app_home = home.join(".rustground");
            if let Err(e) = fs::create_dir_all(&app_home) {
                error!("Unable to create app home: {:?}: {:?}", &app_home, e);
            }
            folders.push(app_home);
        }
        let current_dir = env::current_dir().unwrap_or(PathBuf::from("."));
        info!("Current dir is \"{}\"", current_dir.display());
        folders.push(current_dir.join("base"));
        folders.push(current_dir.join("base/resources"));
        let roots = folders
            .iter()
            .filter(|p| p.exists())
            .map(|path| {
                let r = FileRoot::try_new(path);
                if r.is_err() {
                    r.inspect_err(|error| {
                        warn!("Failed to map \"{}\": {error}", path.display());
                    })
                } else {
                    r.inspect(|root| {
                        info!("Added {root}");
                    })
                }
            })
            .filter(Result::is_ok)
            .map(Result::unwrap)
            .collect();

        AppFiles { roots }
    }
}

impl Files for AppFiles {
    fn open<S: AsRef<str>>(&mut self, path: S) -> Option<File> {
        self.roots.iter_mut().find_map(|r| r.open(path.as_ref()))
    }
}
