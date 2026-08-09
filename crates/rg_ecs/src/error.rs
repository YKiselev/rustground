use std::sync::PoisonError;

use thiserror::Error;

///
/// EntityError
///
#[derive(Debug, Error)]
pub enum EntityError {
    #[error("No such entity!")]
    NotFound,
    #[error("No such archetype!")]
    NoSuchArchetype,
    #[error("Lock is poisoned!")]
    LockPoisoned,
    #[error("Index is out of bounds!")]
    OutOfBounds,
}

impl<T> From<PoisonError<T>> for EntityError {
    fn from(_: PoisonError<T>) -> Self {
        EntityError::LockPoisoned
    }
}
