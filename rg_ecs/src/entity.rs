use std::fmt::Debug;

use crate::{archetype::ArchetypeId, archetype_storage::StorageRowRef};

///
/// EntityId
///
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
#[repr(transparent)]
pub struct EntityId(u32);

impl EntityId {
    pub fn new(id: u32) -> Self {
        EntityId(id)
    }
}

///
/// EntityRef
///
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntityRef {
    pub(crate) archetype: ArchetypeId,
    pub(crate) arch_ref: StorageRowRef,
}

impl EntityRef {
    #[inline]
    pub(crate) fn new(archetype: ArchetypeId, arch_ref: StorageRowRef) -> Self {
        Self {
            archetype,
            arch_ref,
        }
    }
}
