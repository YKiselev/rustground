use std::{
    collections::HashMap,
    fmt::Display,
    hash::{Hash, Hasher},
    marker::PhantomData,
    slice::Iter,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, RwLock,
    },
};

use fxhash::FxHasher32;
use itertools::Itertools;
use once_cell::sync::{self, Lazy};

use crate::{
    archetype::Archetype,
    chunk::Chunk,
    component::{cast, cast_mut, ComponentId, ComponentStorage, TypedComponentStorage},
    entity::EntityId,
    error::EntityError,
};

///
/// Reference to the row in archetype storage
///
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct StorageRowRef {
    chunk_index: u32,
    local_index: u32,
}

impl StorageRowRef {
    pub(crate) fn new(chunk_index: usize, local_index: usize) -> Self {
        Self {
            chunk_index: chunk_index as u32,
            local_index: local_index as u32,
        }
    }

    #[inline(always)]
    pub(crate) fn chunk_index(&self) -> usize {
        self.chunk_index as usize
    }

    #[inline(always)]
    pub(crate) fn local_index(&self) -> usize {
        self.local_index as usize
    }
}

///
/// ArchetypeStorage
///
pub(crate) struct ArchetypeStorage {
    pub(crate) archetype: Archetype,
    chunk_size: usize,
    chunks: Vec<Chunk>,
}

impl ArchetypeStorage {
    /// Creates new archetype storage
    pub(crate) fn new(archetype: Archetype, chunk_size_in_bytes: usize) -> Self {
        let chunk_size = std::cmp::max(1, chunk_size_in_bytes / archetype.row_bytes());
        ArchetypeStorage {
            archetype,
            chunk_size,
            chunks: vec![],
        }
    }

    // Returns (chunk_index, column)
    #[inline]
    pub(crate) fn get_by_type<T>(&mut self) -> Option<(usize, &RwLock<Box<dyn ComponentStorage>>)>
    where
        T: Default + 'static,
    {
        self.get(ComponentId::new::<T>())
    }

    /// Returns column of chunk at [chunk_index]
    #[inline]
    pub(crate) fn get_by_type_at<T>(
        &self,
        chunk_index: usize,
    ) -> Option<&RwLock<Box<dyn ComponentStorage>>>
    where
        T: Default + 'static,
    {
        self.get_at(ComponentId::new::<T>(), chunk_index)
    }

    /// Returns chunk with at least 1 unused row for adding
    fn index_of_available_chunk(&mut self) -> usize {
        let mut index = None;
        for (i, chunk) in self.chunks.iter().enumerate() {
            if chunk.available() > 0 {
                index = Some(i);
                break;
            }
        }
        if index.is_none() {
            // No unfilled chunks (or no chunks at all). Let's add new
            let chunk = self.archetype.new_chunk(self.chunk_size);
            index = Some(self.chunks.len());
            self.chunks.push(chunk);
        }
        index.unwrap()
    }

    ///
    /// Returns column with at least 1 free row
    ///
    #[inline]
    pub(crate) fn get(
        &mut self,
        comp_id: ComponentId,
    ) -> Option<(usize, &RwLock<Box<dyn ComponentStorage>>)> {
        let chunk_index = self.index_of_available_chunk();
        let lock = self.chunks[chunk_index].columns.get(&comp_id)?;
        Some((chunk_index, lock))
    }

    /// Returns column from chunk at index [chunk_index]
    pub(crate) fn get_at(
        &self,
        comp_id: ComponentId,
        chunk_index: usize,
    ) -> Option<&RwLock<Box<dyn ComponentStorage>>> {
        let chunk = self.chunks.get(chunk_index)?;
        chunk.columns.get(&comp_id)
    }

    /// Moves row from this storage to other with additional column's cell value.
    /// Returns new reference to moved entity and and optional id of entity that was swapped with removed one in this storage
    pub(crate) fn move_to<T: Default + 'static>(
        &self,
        dest: &mut ArchetypeStorage,
        arch_ref: &StorageRowRef,
        value: T,
    ) -> Result<(StorageRowRef, Option<EntityId>), EntityError> {
        let chunk = self
            .chunks
            .get(arch_ref.chunk_index())
            .ok_or(EntityError::OutOfBounds)?;
        let dest_ch_num = dest.index_of_available_chunk();
        let dest_chunk = &dest.chunks[dest_ch_num];
        let (new_index, swapped_ent_id) = chunk.move_to(arch_ref.local_index(), dest_chunk, value);
        Ok((StorageRowRef::new(dest_ch_num, new_index), swapped_ent_id))
    }

    /// Adds new row for passed entity to this storage
    pub(crate) fn add(&mut self, ent_id: EntityId) -> StorageRowRef {
        let chunk_index = self.index_of_available_chunk();
        let local_index = self.chunks[chunk_index].add(ent_id);
        StorageRowRef::new(chunk_index, local_index)
    }

    /// Removes row from this storage. Returns id of moved enity (in case of swap remove)
    pub(crate) fn remove(&self, arch_ref: &StorageRowRef) -> Option<EntityId> {
        self.chunks
            .get(arch_ref.chunk_index())
            .and_then(|ch| ch.remove(arch_ref.local_index()))
    }

    /// Gets iterator over chunks of this storage
    pub(crate) fn iter(&self) -> Iter<'_, Chunk> {
        self.chunks.iter()
    }

    /// Removes all rows from this storage
    pub(crate) fn clear(&mut self) {
        self.chunks.clear();
    }

    /// Returns number of rows in this storage
    pub(crate) fn row_count(&self) -> usize {
        self.chunks.iter().map(|chunk| chunk.row_count()).sum()
    }
}

#[cfg(test)]
mod test {

    use crate::{
        archetype_storage::{ArchetypeStorage, StorageRowRef},
        build_archetype,
        entity::EntityId,
    };

    #[test]
    fn add_remove() {
        let mut storage = ArchetypeStorage::new(build_archetype![i32, String, f64, bool], 256);

        assert_eq!(StorageRowRef::new(0, 0), storage.add(EntityId::new(1)));
        assert_eq!(StorageRowRef::new(0, 1), storage.add(EntityId::new(2)));
        assert_eq!(StorageRowRef::new(0, 2), storage.add(EntityId::new(3)));

        storage.remove(&StorageRowRef::new(0, 0));
        storage.remove(&StorageRowRef::new(0, 0));
        storage.remove(&StorageRowRef::new(0, 0));

        assert_eq!(StorageRowRef::new(0, 0), storage.add(EntityId::new(4)));

        storage.get_by_type::<i32>().unwrap();
        storage.get_by_type::<f64>().unwrap();
        storage.get_by_type::<String>().unwrap();
        storage.get_by_type::<bool>().unwrap();
        storage.get_by_type::<EntityId>().unwrap();

        assert!(storage.get_by_type::<i8>().is_none());

        assert!(storage.row_count() > 0);
        storage.clear();
        assert_eq!(0, storage.row_count());
    }

    #[test]
    fn move_to() {
        let e1 = EntityId::new(1);
        let e2 = EntityId::new(2);
        let e3 = EntityId::new(3);
        let e4 = EntityId::new(4);

        // Force many small chunks
        let mut src = ArchetypeStorage::new(build_archetype![String, f64, bool], 1);
        let mut dest = ArchetypeStorage::new(build_archetype![String, f64, bool, i32], 1);

        assert_eq!(StorageRowRef::new(0, 0), src.add(e1));
        assert_eq!(StorageRowRef::new(1, 0), src.add(e2));
        assert_eq!(StorageRowRef::new(2, 0), src.add(e3));
        assert_eq!(StorageRowRef::new(3, 0), src.add(e4));

        assert_eq!(
            (StorageRowRef::new(0, 0), None),
            src.move_to(&mut dest, &StorageRowRef::new(0, 0), 1i32)
                .unwrap()
        );
        assert_eq!(
            (StorageRowRef::new(1, 0), None),
            src.move_to(&mut dest, &StorageRowRef::new(1, 0), 2i32)
                .unwrap()
        );
        assert_eq!(
            (StorageRowRef::new(2, 0), None),
            src.move_to(&mut dest, &StorageRowRef::new(2, 0), 3i32)
                .unwrap()
        );
        assert_eq!(
            (StorageRowRef::new(3, 0), None),
            src.move_to(&mut dest, &StorageRowRef::new(3, 0), 4i32)
                .unwrap()
        );

        assert_eq!(0, src.row_count());
        assert_eq!(4, dest.row_count());

        // Check big chunks
        let mut src = ArchetypeStorage::new(build_archetype![String, f64, bool], 1000);
        let mut dest = ArchetypeStorage::new(build_archetype![String, f64, bool, i32], 1000);

        assert_eq!(StorageRowRef::new(0, 0), src.add(e1));
        assert_eq!(StorageRowRef::new(0, 1), src.add(e2));
        assert_eq!(StorageRowRef::new(0, 2), src.add(e3));
        assert_eq!(StorageRowRef::new(0, 3), src.add(e4));

        assert_eq!(
            (StorageRowRef::new(0, 0), Some(e4)),
            src.move_to(&mut dest, &StorageRowRef::new(0, 0), 1i32)
                .unwrap()
        );
        assert_eq!(
            (StorageRowRef::new(0, 1), Some(e3)),
            src.move_to(&mut dest, &StorageRowRef::new(0, 1), 2i32)
                .unwrap()
        );
        assert_eq!(
            (StorageRowRef::new(0, 2), None),
            src.move_to(&mut dest, &StorageRowRef::new(0, 1), 3i32)
                .unwrap()
        );
        assert_eq!(
            (StorageRowRef::new(0, 3), None),
            src.move_to(&mut dest, &StorageRowRef::new(0, 0), 4i32)
                .unwrap()
        );

        assert_eq!(0, src.row_count());
        assert_eq!(4, dest.row_count());
    }
}
