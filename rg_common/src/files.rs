use std::borrow::Borrow;
use std::fmt::{Display, Formatter};
use std::fs::{File, read};
use std::io::{BufRead, BufReader, Error, ErrorKind, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::{PoisonError, RwLock};
use std::{env, fs};

use log::{debug, error, info, warn};
use thiserror::Error;

use crate::arguments::Arguments;

///
/// File error
///
#[derive(Debug, Error)]
pub enum FileError {
    #[error("I/O error {0}")]
    IoError(#[from] std::io::Error),
    #[error("Lock poisoned")]
    LockPoisoned,
}

impl<T> From<PoisonError<T>> for FileError {
    fn from(_: PoisonError<T>) -> Self {
        FileError::LockPoisoned
    }
}

///
/// Readable and Writable resources
///
pub trait SeekAndRead: Read + Seek {}

pub trait SeekAndWrite: Write + Seek {}

impl SeekAndRead for File {}

impl SeekAndWrite for File {}

///
/// File root
///
#[derive(Debug)]
struct FileRoot {
    readonly: bool,
    path: PathBuf,
}

impl FileRoot {
    fn try_new(path: &Path) -> Result<Self, FileError> {
        if !path.exists() {
            return Err(FileError::IoError(Error::from(ErrorKind::NotFound)));
        }
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

    fn read(&self, path: &str) -> Result<Box<dyn SeekAndRead + Send>, FileError> {
        let mut buf = self.path.clone();
        buf.push(path);
        match File::open(&buf) {
            Ok(file) => {
                debug!("open({:?})", &buf);
                Ok(Box::new(file))
            }
            Err(e) => {
                debug!("File not found: {:?}, {:?}", buf, e);
                Err(FileError::IoError(e))
            }
        }
    }

    fn write(&self, path: &str) -> Result<Box<dyn SeekAndWrite + Send>, FileError> {
        let mut buf = self.path.clone();
        buf.push(path);
        match File::create(&buf) {
            Ok(file) => {
                debug!("create({:?})", &file);
                Ok(Box::new(file))
            }
            Err(e) => {
                warn!("Unable to create file: {:?}, {:?}", buf, e);
                Err(FileError::IoError(e))
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
            .filter_map(|path| match FileRoot::try_new(path) {
                Ok(root) => {
                    info!("Added {}", root);
                    Some(root)
                }
                Err(e) => {
                    warn!("Failed to map {:?}: {:?}", path, e);
                    None
                }
            })
            .collect();

        Files {
            roots: RwLock::new(roots),
        }
    }

    pub fn read<S>(&self, path: S) -> Result<Box<dyn SeekAndRead + Send>, FileError>
    where
        S: AsRef<str>,
    {
        let guard = self.roots.read()?;
        let path_str = path.as_ref();
        let mut last_err: Option<FileError> = None;
        for root in guard.iter() {
            match root.read(path_str) {
                Ok(f) => return Ok(f),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| FileError::IoError(Error::from(ErrorKind::NotFound))))
    }

    pub fn buf_read<S>(&self, path: S) -> Result<Box<dyn BufRead + Send>, FileError>
    where
        S: AsRef<str>,
    {
        Ok(self.read(path).map(BufReader::new).map(Box::new)?)
    }

    pub fn write<S>(&self, path: S) -> Result<Box<dyn SeekAndWrite + Send>, FileError>
    where
        S: AsRef<str>,
    {
        let guard = self.roots.read()?;
        let path_str = path.as_ref();
        let mut last_err: Option<FileError> = None;
        for root in guard.iter() {
            match root.write(path_str) {
                Ok(f) => return Ok(f),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| FileError::IoError(Error::from(ErrorKind::NotFound))))
    }

    ///
    /// Reads small file into string
    ///
    pub fn read_file<S>(&self, name: S) -> Result<String, FileError>
    where
        S: AsRef<str>,
    {
        let mut file = self.read(name)?;
        let mut tmp = String::new();
        file.read_to_string(&mut tmp)?;
        Ok(tmp)
    }

    ///
    /// Writes small string to file
    ///
    pub fn write_file<S>(&self, name: &str, value: S)
    where
        S: Borrow<str>,
    {
        if let Ok(mut file) = self.write(name) {
            match write!(file, "{}", value.borrow()) {
                Ok(_) => {
                    file.flush().unwrap();
                }
                Err(e) => {
                    warn!("Unable to write to file: {:?}", e)
                }
            }
        }
    }
}
