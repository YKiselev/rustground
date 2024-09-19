use std::sync::PoisonError;

use snafu::Snafu;

///
/// EntityError
///
#[derive(Debug, Snafu)]
pub enum EntityError {
    #[snafu(display("No such entity!"))]
    NotFound,
    #[snafu(display("No such archetype!"))]
    NotSuchArchetype,
    #[snafu(display("Lock is poisoned!"))]
    LockPoisoned
}

impl<T> From<PoisonError<T>> for EntityError {
    fn from(_: PoisonError<T>) -> Self {
        EntityError::LockPoisoned
    }
}
