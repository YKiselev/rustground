use std::{
    any::{Any, TypeId},
    borrow::Borrow,
    collections::{HashMap, hash_map::Entry},
    fmt::Display,
    io::Read,
    sync::{Arc, PoisonError, RwLock, Weak},
};

use thiserror::Error;

use crate::{Loader, LoaderError};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct Key(Box<str>, TypeId);

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Key({},{:?})", self.0, self.1)
    }
}

impl Key {
    fn new<S, T>(name: S) -> Self
    where
        S: Into<Box<str>>,
        T: 'static,
    {
        Self(name.into(), TypeId::of::<T>())
    }
}

type Erased = dyn Any + Send + Sync;
type TypeMap = HashMap<Key, Weak<Erased>>;

pub struct Assets(RwLock<TypeMap>);

impl Assets {
    pub fn new() -> Self {
        Self(RwLock::new(TypeMap::new()))
    }

    pub fn load<S, R, L, A, Rd>(
        &self,
        name: S,
        resolver: R,
        loader: &L,
    ) -> Result<Arc<A>, AssetError>
    where
        A: Send + Sync + 'static,
        S: Into<Box<str>> + Borrow<str>,
        Rd: Read,
        R: Fn(&str) -> Option<Rd>,
        L: Loader<A, Rd> + 'static,
    {
        let guard = self.0.read()?;
        let key = Key::new::<_, L>(name);
        if let Some(erased) = guard.get(&key).and_then(|weak| weak.upgrade()) {
            if let Ok(asset) = Arc::downcast::<A>(erased) {
                return Ok(Arc::clone(&asset));
            }
        }
        drop(guard);
        // Load asset w/o any locks!
        let mut reader = (resolver)(key.0.borrow()).ok_or(AssetError::NotFound)?;
        let asset = loader(&mut reader as _)?;
        let asset = Arc::new(asset);

        let mut guard = self.0.write()?;
        let typed = match guard.entry(key) {
            Entry::Occupied(mut entry) => match entry.get().upgrade() {
                Some(erased) => Arc::downcast::<A>(erased)
                    .map_err(|_| AssetError::TypeMismatch(entry.key().clone()))?,
                _ => {
                    entry.insert(downgrade(&asset));
                    asset
                }
            },
            Entry::Vacant(entry) => {
                entry.insert(downgrade(&asset));
                asset
            }
        };
        Ok(typed)
    }
}

#[inline]
fn downgrade<A>(value: &Arc<A>) -> Weak<Erased>
where
    A: Send + Sync + 'static,
{
    Arc::downgrade(value) as Weak<Erased>
}

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("Lock poisoned")]
    LockPoisoned,
    #[error("Not found")]
    NotFound,
    #[error("Type mismatch for key {0}")]
    TypeMismatch(Key),
    #[error("{0}")]
    Loader(LoaderError),
}

impl<T> From<PoisonError<T>> for AssetError {
    fn from(_: PoisonError<T>) -> Self {
        AssetError::LockPoisoned
    }
}

impl From<LoaderError> for AssetError {
    fn from(value: LoaderError) -> Self {
        AssetError::Loader(value)
    }
}

#[cfg(test)]
mod tests {

    use std::io::BufReader;

    use super::*;

    static mut L1_COUNTER: i32 = 0;

    fn loader_1<R>(read: &mut R) -> Result<String, LoaderError>
    where
        R: Read,
    {
        unsafe { L1_COUNTER += 1 };

        Ok(String::from("value"))
    }

    #[test]
    fn asset_loaders() {
        let resolver = |_: &str| Some(BufReader::new(b"test" as &[u8]));
        let assets = Assets::new();
        let first = assets.load("first", &resolver, &loader_1).unwrap();
        let second = assets.load("first", &resolver, &loader_1).unwrap();
        let third = assets.load("first", &resolver, &loader_1).unwrap();
        assert_eq!(1, unsafe { L1_COUNTER });
    }
}
