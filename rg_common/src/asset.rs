use std::{
    any::{Any, TypeId},
    borrow::{Borrow, Cow},
    collections::{hash_map::Entry, HashMap, HashSet},
    fmt::Display,
    fs::File,
    io::{BufRead, BufReader, Read},
    marker::PhantomData,
    sync::{Arc, PoisonError, RwLock, Weak},
};

use snafu::Snafu;
use uuid::Uuid;

use crate::App;

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

pub trait Resolver<R>: Fn(&str) -> Option<R>
where
    R: Read,
{
}

impl<R, T> Resolver<R> for T
where
    R: Read,
    T: Fn(&str) -> Option<R>,
{
}

pub trait Loader<A, R>: Fn(&R) -> Result<Arc<A>, AssetError>
where
    A: Send + Sync,
    R: Read,
{
}

impl<A, R, T> Loader<A, R> for T
where
    A: Send + Sync,
    R: Read,
    T: Fn(&R) -> Result<Arc<A>, AssetError>,
{
}

pub struct Assets<R, Rd>(R, RwLock<TypeMap>, PhantomData<Rd>);

impl<R, Rd> Assets<R, Rd>
where
    Rd: Read,
    R: Resolver<Rd>,
{
    pub fn new(resolver: R) -> Self
    {
        Self(
            resolver,
            RwLock::new(TypeMap::new()),
            PhantomData::default(),
        )
    }

    pub fn load<S, L, A>(&self, name: S, loader: L) -> Result<Arc<A>, AssetError>
    where
        A: Send + Sync + 'static,
        S: Into<Box<str>> + Borrow<str>,
        L: Loader<A, Rd> + 'static,
    {
        let guard = self.1.read()?;
        let key = Key::new::<_, L>(name);
        if let Some(erased) = guard.get(&key).and_then(|weak| weak.upgrade()) {
            if let Ok(asset) = Arc::downcast::<A>(erased) {
                return Ok(Arc::clone(&asset));
            }
        }
        // Load asset under read guard
        let reader = (self.0)(key.0.borrow()).ok_or(AssetError::NotFound)?;
        let asset = loader(&reader as _)?;
        drop(guard);
        let mut guard = self.1.write()?;
        let typed = match guard.entry(key) {
            Entry::Occupied(mut entry) => {
                if let Some(erased) = entry.get().upgrade() {
                    Arc::downcast::<A>(erased).map_err(|_| AssetError::TypeMismatch {
                        key: entry.key().clone(),
                    })?
                } else {
                    entry.insert(downgrade(&asset));
                    asset
                }
            }
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

#[derive(Debug, Snafu)]
pub enum AssetError {
    #[snafu(display("Lock poisoned"))]
    LockPoisoned,
    #[snafu(display("Not found"))]
    NotFound,
    #[snafu(display("Key already registered: {uuid}"))]
    KeyAlreadyRegistered { uuid: Uuid },
    #[snafu(display("Bad key: {uuid}"))]
    BadKey { uuid: Uuid },
    #[snafu(display("Type mismatch for key {key}"))]
    TypeMismatch { key: Key },
}

impl<T> From<PoisonError<T>> for AssetError {
    fn from(_: PoisonError<T>) -> Self {
        AssetError::LockPoisoned
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    static mut L1_COUNTER: i32 = 0;

    fn loader_1<R>(read: &R) -> Result<Arc<String>, AssetError>
    where
        R: Read,
    {
        unsafe { L1_COUNTER += 1 };

        Ok(Arc::new(String::from("value")))
    }

    #[test]
    fn asset_loaders() {
        let resolver = |_:&str| {
            Some(b"test" as &[u8])
        };
        let assets = Assets::new(resolver);

        let first = assets.load("first", loader_1).unwrap();
        let second = assets.load("first", loader_1).unwrap();
        let third = assets.load("first", loader_1).unwrap();
        assert_eq!(1, unsafe { L1_COUNTER });
    }
}
