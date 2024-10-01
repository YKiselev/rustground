use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
    slice::Iter,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
};

use itertools::Itertools;
use once_cell::sync::{self, Lazy};

use crate::{
    component::{cast, cast_mut, ComponentId, ComponentStorage, TypedComponentStorage},
    entity::EntityId,
};
///
/// Constants
///
pub(crate) static COLUMN_ENTITY_ID: Lazy<ComponentId> =
    sync::Lazy::new(|| ComponentId::new::<EntityId>());

///
/// ArchetypeId
///
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct ArchetypeId(u64);

impl ArchetypeId {
    pub fn new<'a>(components: impl Iterator<Item = &'a ComponentId>) -> Self {
        let mut hasher = DefaultHasher::new();
        for c in components {
            c.hash(&mut hasher);
        }
        ArchetypeId(hasher.finish())
    }
}

impl Display for ArchetypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ArchetypeId({})", self.0)
    }
}

///
/// ColumnFactory
///
type ColumnFactory = dyn Fn(usize) -> Box<dyn ComponentStorage + 'static>;

///
/// Archetype builder
///
pub struct ArchetypeBuilder {
    factories: HashMap<ComponentId, Arc<ColumnFactory>>,
}

impl ArchetypeBuilder {
    pub fn new() -> Self {
        ArchetypeBuilder {
            factories: HashMap::with_capacity(4),
        }
        .add::<EntityId>()
    }

    pub fn add<T: Default + 'static>(mut self) -> Self {
        let comp_id = ComponentId::new::<T>();
        self.factories.insert(
            comp_id,
            Arc::new(|capacity| Box::new(TypedComponentStorage::<T>::new(capacity))),
        );
        self
    }

    pub fn build(self) -> Archetype {
        let mut hasher = DefaultHasher::new();
        for id in self.factories.keys().sorted() {
            id.hash(&mut hasher);
        }
        Archetype {
            id: ArchetypeId(hasher.finish()),
            factories: self.factories,
        }
    }
}

///
/// Archetype
///
#[derive(Clone)]
pub struct Archetype {
    pub id: ArchetypeId,
    factories: HashMap<ComponentId, Arc<ColumnFactory>>,
}

impl Archetype {
    fn new_chunk(&self, capacity: usize) -> Chunk {
        Chunk::new(
            self.factories
                .iter()
                .map(|(id, f)| (*id, RwLock::new((f)(capacity))))
                .collect(),
            capacity,
        )
    }

    pub fn to_builder(&self) -> ArchetypeBuilder {
        ArchetypeBuilder {
            factories: self
                .factories
                .iter()
                .map(|(id, f)| (*id, Arc::clone(f)))
                .collect(),
        }
    }

    pub fn has_component(&self, comp_id: &ComponentId) -> bool {
        self.factories.contains_key(comp_id)
    }
}

impl PartialEq for Archetype {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Archetype {}

impl Display for Archetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Archetype(id={}, size={})",
            self.id,
            self.factories.len()
        )
    }
}

impl std::fmt::Debug for Archetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Archetype").field("id", &self.id).finish()
    }
}

///
/// Chunk
///
type ColumnMap = HashMap<ComponentId, RwLock<Box<dyn ComponentStorage>>>;

pub struct Chunk {
    columns: ColumnMap,
    available_rows: AtomicUsize,
}

impl Chunk {
    fn new(columns: ColumnMap, size: usize) -> Self {
        Chunk {
            columns,
            available_rows: AtomicUsize::new(size),
        }
    }

    fn available(&self) -> usize {
        self.available_rows.load(Ordering::Acquire)
    }

    ///
    /// Adds new row for passed entity to this storage and returns local index
    ///
    fn add(&self, ent_id: EntityId) -> usize {
        assert!(self.available() > 0);
        let mut index = 0;
        for (id, column) in self.columns.iter() {
            let mut guard = column.write().unwrap();
            if *id == *COLUMN_ENTITY_ID {
                index = cast_mut::<EntityId>(guard.as_mut()).push(ent_id);
            } else {
                index = guard.add();
            }
        }
        self.available_rows.fetch_sub(1, Ordering::Relaxed);
        index
    }

    fn get_entity_id(&self, index: usize) -> Option<EntityId> {
        let column = self.columns.get(&COLUMN_ENTITY_ID)?;
        cast::<EntityId>(column.read().unwrap().as_ref())
            .get(index)
            .copied()
    }

    ///
    /// Removes row from this chunk
    ///
    fn remove(&self, index: usize) -> Option<EntityId> {
        for column in self.columns.values() {
            column.write().unwrap().remove(index);
        }
        self.available_rows.fetch_add(1, Ordering::Relaxed);
        self.get_entity_id(index)
    }

    ///
    /// Moves row from this chunk to another storage. Returns id of the entity which has taken place of the moved one.
    ///
    fn move_to(&self, dest: &mut ArchetypeStorage, index: usize) -> Option<EntityId> {
        for (comp_id, column) in self.columns.iter() {
            column
                .write()
                .unwrap()
                .move_to(index, dest.get(*comp_id).unwrap().write().unwrap().as_mut());
        }
        self.get_entity_id(index)
    }

    pub(crate) fn get_column(
        &self,
        comp_id: ComponentId,
    ) -> Option<&RwLock<Box<dyn ComponentStorage>>> {
        self.columns.get(&comp_id)
    }

    fn row_count(&self) -> usize {
        for (_, col) in self.columns.iter() {
            return col.read().unwrap().row_count();
        }
        0
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
    pub(crate) fn new(archetype: Archetype, chunk_size: usize) -> Self {
        ArchetypeStorage {
            archetype,
            chunk_size,
            chunks: vec![],
        }
    }

    #[inline]
    pub(crate) fn get_by_type<T>(&mut self) -> Option<&RwLock<Box<dyn ComponentStorage>>>
    where
        T: Default + 'static,
    {
        self.get(ComponentId::new::<T>())
    }

    #[inline]
    pub(crate) fn get_by_type_at<T>(
        &self,
        index: usize,
    ) -> Option<(&RwLock<Box<dyn ComponentStorage>>, usize)>
    where
        T: Default + 'static,
    {
        self.get_at(ComponentId::new::<T>(), index)
    }

    ///
    /// Returns chunk with at least 1 unused row for adding
    ///
    fn index_of_available_chunk(&mut self) -> Option<usize> {
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
        index
    }

    ///
    /// Returns column with at least 1 free row
    ///
    #[inline]
    pub(crate) fn get(
        &mut self,
        comp_id: ComponentId,
    ) -> Option<&RwLock<Box<dyn ComponentStorage>>> {
        let chunk_index = self.index_of_available_chunk()?;
        self.chunks[chunk_index].columns.get(&comp_id)
    }

    pub(crate) fn get_at(
        &self,
        comp_id: ComponentId,
        index: usize,
    ) -> Option<(&RwLock<Box<dyn ComponentStorage>>, usize)> {
        let (ch_num, local_index) = self.to_local(index);
        let chunk = self.chunks.get(ch_num)?;
        chunk.columns.get(&comp_id).map(|c| (c, local_index))
    }

    #[inline(always)]
    fn to_local(&self, index: usize) -> (usize, usize) {
        (index / self.chunk_size, index % self.chunk_size)
    }

    pub(crate) fn move_to<T: Default + 'static>(
        &self,
        dest: &mut ArchetypeStorage,
        index: usize,
        value: T,
    ) -> (usize, Option<EntityId>) {
        let (ch_num, local_index) = self.to_local(index);
        let chunk = &self.chunks[ch_num];
        let swapped_ent_id = chunk.move_to(dest, local_index);
        (
            cast_mut::<T>(dest.get_by_type::<T>().unwrap().write().unwrap().as_mut()).push(value),
            swapped_ent_id,
        )
    }

    ///
    /// Adds new row for passed entity to this storage
    ///
    pub(crate) fn add(&mut self, ent_id: EntityId) -> usize {
        let chunk_index = self.index_of_available_chunk().unwrap();
        let local_index = self.chunks[chunk_index].add(ent_id);
        chunk_index * self.chunk_size + local_index
    }

    ///
    /// Removes row from this storage. Returns id of moved enity (in case of swap remove)
    ///
    pub(crate) fn remove(&self, index: usize) -> Option<EntityId> {
        let (ch_num, local_index) = self.to_local(index);
        if let Some(chunk) = self.chunks.get(ch_num) {
            return chunk.remove(local_index);
        }
        None
    }

    ///
    /// Gets iterator over chunks of this storage
    ///
    pub(crate) fn iter(&self) -> Iter<'_, Chunk> {
        self.chunks.iter()
    }

    ///
    /// Removes all rows from this storage
    ///
    pub(crate) fn clear(&mut self) {
        self.chunks.clear();
    }

    ///
    /// Returns number of rows in this storage
    ///
    pub(crate) fn row_count(&self) -> usize {
        self.chunks.iter().map(|chunk| chunk.row_count()).sum()
    }
}

///
/// Macros
///
#[macro_export]
#[doc(hidden)]
macro_rules! build_archetype {
    ($($column_type:ty),*) => {
        $crate::archetype::ArchetypeBuilder::new()
        $(.add::<$column_type>())*
        .build()
    };
}

pub use build_archetype;

///
/// Tests
///
#[cfg(test)]
mod test {
    use std::hash::{DefaultHasher, Hash, Hasher};

    use crate::{
        archetype::ArchetypeStorage,
        component::{ComponentId, ComponentStorage},
        entity::EntityId,
    };

    #[test]
    fn archetype_to_builder() {
        let archetype = build_archetype![i32, String, f64, bool];
        let builder = archetype.to_builder();
        let archetype2 = builder.build();
        let builder = archetype2.to_builder();
        let archetype3 = builder.build();
        assert_eq!(archetype, archetype2);
        assert_eq!(archetype, archetype3);
    }

    #[test]
    fn test() {
        let mut storage = ArchetypeStorage::new(build_archetype![i32, String, f64, bool], 3);

        assert_eq!(0, storage.add(EntityId(1)));
        assert_eq!(1, storage.add(EntityId(2)));
        assert_eq!(2, storage.add(EntityId(3)));

        storage.remove(0);
        storage.remove(1);
        storage.remove(0);

        assert_eq!(0, storage.add(EntityId(4)));

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
}
