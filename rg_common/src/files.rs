use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Error;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::{env, fs};

use log::{debug, error, info, warn};

use crate::arguments::Arguments;

pub trait Files {
    fn read<S: AsRef<str>>(&self, path: S) -> Option<File>;
    fn write<S: AsRef<str>>(&self, path: S) -> Option<File>;
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

    fn read(&self, path: &str) -> Option<File> {
        let mut buf = self.path.clone();
        buf.push(path);
        match File::open(&buf) {
            Ok(file) => {
                debug!("open({:?})", &buf);
                Some(file)
            }
            Err(e) => {
                debug!("File not found: {:?}, {:?}", buf, e);
                None
            }
        }
    }

    fn write(&self, path: &str) -> Option<File> {
        let mut buf = self.path.clone();
        buf.push(path);
        match File::create(&buf) {
            Ok(file) => {
                debug!("create({:?})", &file);
                Some(file)
            }
            Err(e) => {
                warn!("Unable to create file: {:?}, {:?}", buf, e);
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
    roots: RwLock<Vec<FileRoot>>,
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

        AppFiles {
            roots: RwLock::new(roots),
        }
    }
}

impl Files for AppFiles {
    fn read<S: AsRef<str>>(&self, path: S) -> Option<File> {
        let guard = self.roots.read().ok()?;
        guard.iter().find_map(|r| r.read(path.as_ref()))
    }

    fn write<S: AsRef<str>>(&self, path: S) -> Option<File> {
        let guard = self.roots.read().ok()?;
        guard.iter().find_map(|r| r.write(path.as_ref()))
    }
}
