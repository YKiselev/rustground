use std::borrow::Borrow;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Error, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::{env, fs};

use log::{debug, error, info, warn};

use crate::arguments::Arguments;

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

const HOME_DIR: &str = ".rustground";

#[derive(Debug)]
pub struct Files {
    roots: RwLock<Vec<FileRoot>>,
}

impl Files {
    pub fn new(args: &Arguments) -> Self {
        let mut folders: Vec<PathBuf> = Vec::new();
        if let Some(home) = dirs::home_dir() {
            let app_home = home.join(HOME_DIR);
            if let Err(e) = fs::create_dir_all(&app_home) {
                error!("Unable to create app home: {:?}: {:?}", &app_home, e);
            }
            folders.push(app_home);
        }
        let current_dir = env::current_dir().expect("Unable to get current directory!");
        info!("Current dir is {:?}", current_dir);
        folders.push(current_dir.join("base"));
        folders.push(current_dir.join("base/resources"));
        let roots = folders
            .iter()
            .filter(|p| p.exists())
            .filter_map(|path| {
                match FileRoot::try_new(path) {
                    Ok(root) => {
                        info!("Added {}", root);
                        Some(root)
                    },
                    Err(e) => {
                        warn!("Failed to map {:?}: {:?}", path, e);
                        None
                    }
                }
            })
            .collect();

        Files {
            roots: RwLock::new(roots),
        }
    }

    pub fn read<S>(&self, path: S) -> Option<File>
    where
        S: Borrow<str>,
    {
        let guard = self.roots.read().ok()?;
        guard.iter().find_map(|r| r.read(path.borrow()))
    }

    pub fn write<S>(&self, path: S) -> Option<File>
    where
        S: AsRef<str>,
    {
        let guard = self.roots.read().ok()?;
        guard.iter().find_map(|r| r.write(path.as_ref()))
    }

    ///
    /// Reads small file into string
    /// 
    pub fn read_file<S>(&self, name: S) -> Option<String>
    where
        S: Borrow<str>,
    {
        let mut cfg = self.read(name)?;
        let mut tmp = String::new();
        cfg.read_to_string(&mut tmp).ok()?;
        Some(tmp)
    }

    ///
    /// Writes small string to file
    /// 
    pub fn write_file<S>(&self, name: &str, value: S)
    where
        S: Borrow<str>,
    {
        if let Some(mut file) = self.write(name) {
            match write!(file, "{}", value.borrow()) {
                Ok(_) => {
                    file.flush().unwrap();
                }
                Err(e) => {
                    warn!("Unable to save config: {:?}", e)
                }
            }
        }
    }
}
