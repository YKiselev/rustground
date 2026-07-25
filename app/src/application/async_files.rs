use std::{io::ErrorKind, path::PathBuf};

use bytes::{Bytes, BytesMut};
use rg_common::FileRoot;
use tokio::{fs::File, io::AsyncReadExt};
use tracing::warn;

use crate::error::AppError;

pub struct AsyncFiles {
    roots: Vec<FileRoot>,
}

impl AsyncFiles {
    pub fn new(roots: &Vec<FileRoot>) -> Self {
        Self {
            roots: roots.to_vec(),
        }
    }

    pub async fn load_file<S>(&self, name: S, mut buf: BytesMut) -> Result<Bytes, AppError>
    where
        S: AsRef<str>,
    {
        let mut path_buf = PathBuf::default();
        for root in self.roots.iter() {
            path_buf.clear();

            match load_file_from_root(root, name.as_ref(), &mut path_buf, &mut buf).await {
                Ok(result) => {
                    return Ok(result);
                }
                Err(AppError::IoError(kind)) if kind == ErrorKind::NotFound => {}
                Err(e) => {
                    warn!("Failed to resolve file {}: {:?}", name.as_ref(), e);
                }
            }
        }
        Err(AppError::IoError(std::io::ErrorKind::NotFound))
    }
}

async fn load_file_from_root<S>(
    root: &FileRoot,
    name: S,
    path_buf: &mut PathBuf,
    buf: &mut BytesMut,
) -> Result<Bytes, AppError>
where
    S: AsRef<str>,
{
    path_buf.push(&root.path);
    path_buf.push(name.as_ref());

    let mut file = File::open(path_buf).await?;

    let metadata = file.metadata().await?;
    let file_size = metadata.len() as usize;

    buf.clear();
    buf.reserve(file_size);

    while file.read_buf(buf).await? > 0 {
        // read data
    }

    let result = buf.split();

    Ok(result.freeze())
}
