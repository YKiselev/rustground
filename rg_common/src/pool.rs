use std::sync::{Mutex, PoisonError};

use snafu::Snafu;

pub struct Pool<V, F>
where
    F: Fn() -> V,
{
    pool: Mutex<Vec<V>>,
    factory: F,
}

impl<V, F> Pool<V, F>
where
    F: Fn() -> V,
{
    pub fn new(capacity: usize, factory: F) -> Self {
        Self {
            pool: Mutex::new(Vec::with_capacity(capacity)),
            factory
        }
    }

    pub fn borrow(&self) -> Result<V, PoolError> {
        //let x = self.pool.lock()?.pop().or_else(f);
        unimplemented!()
    }

    pub fn release(v: V) {}
}

#[derive(Debug, Snafu)]
pub enum PoolError {
    #[snafu(display("Lock poisoned"))]
    LockPoisoned,
}

impl<G> From<PoisonError<G>> for PoolError {
    fn from(value: PoisonError<G>) -> Self {
        PoolError::LockPoisoned
    }
}

#[cfg(test)]
mod tests {
    use super::Pool;

    #[test]
    fn test() {
        //let mut pool = Pool::<u32>::default();
    }
}
