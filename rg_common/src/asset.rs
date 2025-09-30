use std::{
    any::{Any, TypeId},
    borrow::Borrow,
    collections::{hash_map::Entry, HashMap},
    fmt::Display,
    io::Read,
    sync::{Arc, PoisonError, RwLock, Weak},
};

use snafu::Snafu;

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
        let reader = (resolver)(key.0.borrow()).ok_or(AssetError::NotFound)?;
        let asset = loader(&reader as _)?;

        let mut guard = self.0.write()?;
        let typed = match guard.entry(key) {
            Entry::Occupied(mut entry) => match entry.get().upgrade() {
                Some(erased) => {
                    Arc::downcast::<A>(erased).map_err(|_| AssetError::TypeMismatch {
                        key: entry.key().clone(),
                    })?
                }
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

#[derive(Debug, Snafu)]
pub enum AssetError {
    #[snafu(display("Lock poisoned"))]
    LockPoisoned,
    #[snafu(display("Not found"))]
    NotFound,
    // #[snafu(display("Key already registered: {uuid}"))]
    // KeyAlreadyRegistered { uuid: Uuid },
    // #[snafu(display("Bad key: {uuid}"))]
    // BadKey { uuid: Uuid },
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

    use std::io::BufReader;

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
        let resolver = |_: &str| Some(BufReader::new(b"test" as &[u8]));
        let assets = Assets::new();
        let first = assets.load("first", &resolver, &loader_1).unwrap();
        let second = assets.load("first", &resolver, &loader_1).unwrap();
        let third = assets.load("first", &resolver, &loader_1).unwrap();
        assert_eq!(1, unsafe { L1_COUNTER });
    }
}
