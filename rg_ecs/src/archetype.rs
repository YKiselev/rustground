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
    component::{cast, cast_mut, ComponentId, ComponentStorage, TypedComponentStorage},
    entity::EntityId,
    error::EntityError,
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
#[repr(transparent)]
pub struct ArchetypeId(u32);

impl ArchetypeId {
    pub fn new<'a>(components: impl Iterator<Item = &'a ComponentId>) -> Self {
        let mut hasher = FxHasher32::default();
        for c in components {
            c.hash(&mut hasher);
        }
        ArchetypeId(hasher.finish() as u32)
    }
}

impl Display for ArchetypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ArchetypeId({})", self.0)
    }
}

///
/// Reference to the row in archetype storage
///
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ArchetypeRef {
    chunk_index: u32,
    local_index: u32,
}

impl ArchetypeRef {
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
/// ColumnFactory
///
trait ColumnFactory {
    fn create(&self, capacity: usize) -> Box<dyn ComponentStorage + 'static>;
    fn item_size(&self) -> usize;
}

#[derive(Default)]
struct TypedColumnFactory<T>
where
    T: Default + 'static,
{
    _data: PhantomData<T>,
}

impl<T> ColumnFactory for TypedColumnFactory<T>
where
    T: Default + 'static,
{
    fn create(&self, capacity: usize) -> Box<dyn ComponentStorage + 'static> {
        Box::new(TypedComponentStorage::<T>::with_capacity(capacity))
    }

    fn item_size(&self) -> usize {
        size_of::<T>()
    }
}

///
/// Archetype builder
///
pub struct ArchetypeBuilder {
    factories: HashMap<ComponentId, Arc<dyn ColumnFactory>>,
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
        self.factories
            .insert(comp_id, Arc::new(TypedColumnFactory::<T>::default()));
        self
    }

    pub fn build(self) -> Archetype {
        let mut hasher = FxHasher32::default();
        for id in self.factories.keys().sorted() {
            id.hash(&mut hasher);
        }
        Archetype {
            id: ArchetypeId(hasher.finish() as u32),
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
    factories: HashMap<ComponentId, Arc<dyn ColumnFactory>>,
}

impl Archetype {
    fn new_chunk(&self, capacity: usize) -> Chunk {
        Chunk::new(
            self.factories
                .iter()
                .map(|(id, f)| (*id, RwLock::new(f.create(capacity))))
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

    pub fn row_bytes(&self) -> usize {
        self.factories.iter().map(|(_, f)| f.item_size()).sum()
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
    available_rows: AtomicU32,
}

impl Chunk {
    ///
    /// Wraps supplied columns in this chunk.
    /// # Arguments:
    /// * `capacity` - the capacity of each supplied column
    ///
    fn new(columns: ColumnMap, capacity: usize) -> Self {
        Chunk {
            columns,
            available_rows: AtomicU32::new(capacity as u32),
        }
    }

    fn available(&self) -> u32 {
        self.available_rows.load(Ordering::Acquire)
    }

    ///
    /// Adds new row for passed entity to this storage and returns local index
    ///
    fn add(&self, ent_id: EntityId) -> usize {
        assert!(self.available() > 0);
        let mut index = 0;
        for (_, column) in self.columns.iter() {
            index = column.write().unwrap().add();
        }
        if let Some(column) = self.columns.get(&COLUMN_ENTITY_ID) {
            // Cell already added in above loop, now set value
            cast_mut::<EntityId>(column.write().unwrap().as_mut())[index] = ent_id;
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
    fn move_to<T>(&self, index: usize, dest: &Chunk, value: T) -> (usize, Option<EntityId>)
    where
        T: Default + 'static,
    {
        for (comp_id, column) in self.columns.iter() {
            let lock = dest.get_column(*comp_id).unwrap();
            let mut guard = lock.write().unwrap();
            column.write().unwrap().move_to(index, guard.as_mut());
        }
        let lock = dest.get_column_for_type::<T>().unwrap();
        let idx = {
            let mut guard = lock.write().unwrap();
            let typed_col = cast_mut::<T>(guard.as_mut());
            let idx = typed_col.len();
            typed_col.push(value);
            idx
        };
        dest.available_rows.fetch_sub(1, Ordering::Relaxed);
        self.available_rows.fetch_add(1, Ordering::Relaxed);
        (idx, self.get_entity_id(index))
    }

    #[inline(always)]
    pub(crate) fn get_column(
        &self,
        comp_id: ComponentId,
    ) -> Option<&RwLock<Box<dyn ComponentStorage>>> {
        self.columns.get(&comp_id)
    }

    #[inline(always)]
    pub(crate) fn get_column_for_type<T>(&self) -> Option<&RwLock<Box<dyn ComponentStorage>>>
    where
        T: Default + 'static,
    {
        self.columns.get(&ComponentId::new::<T>())
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

    ///
    /// Returns chunk with at least 1 unused row for adding
    ///
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

    pub(crate) fn get_at(
        &self,
        comp_id: ComponentId,
        chunk_index: usize,
    ) -> Option<&RwLock<Box<dyn ComponentStorage>>> {
        let chunk = self.chunks.get(chunk_index)?;
        chunk.columns.get(&comp_id)
    }

    ///
    /// Moves row from this storage to other with additional column's cell value.
    /// Returns new reference to moved entity and and optional id of entity that was swapped with removed one in this storage
    ///
    pub(crate) fn move_to<T: Default + 'static>(
        &self,
        dest: &mut ArchetypeStorage,
        arch_ref: &ArchetypeRef,
        value: T,
    ) -> Result<(ArchetypeRef, Option<EntityId>), EntityError> {
        let chunk = self
            .chunks
            .get(arch_ref.chunk_index())
            .ok_or(EntityError::OutOfBounds)?;
        let dest_ch_num = dest.index_of_available_chunk();
        let dest_chunk = &dest.chunks[dest_ch_num];
        let (new_index, swapped_ent_id) = chunk.move_to(arch_ref.local_index(), dest_chunk, value);
        Ok((ArchetypeRef::new(dest_ch_num, new_index), swapped_ent_id))
    }

    ///
    /// Adds new row for passed entity to this storage
    ///
    pub(crate) fn add(&mut self, ent_id: EntityId) -> ArchetypeRef {
        let chunk_index = self.index_of_available_chunk();
        let local_index = self.chunks[chunk_index].add(ent_id);
        ArchetypeRef::new(chunk_index, local_index)
    }

    ///
    /// Removes row from this storage. Returns id of moved enity (in case of swap remove)
    ///
    pub(crate) fn remove(&self, arch_ref: &ArchetypeRef) -> Option<EntityId> {
        self.chunks
            .get(arch_ref.chunk_index())
            .and_then(|ch| ch.remove(arch_ref.local_index()))
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

    use crate::{
        archetype::{ArchetypeRef, ArchetypeStorage},
        component::ComponentStorage,
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
    fn add_remove() {
        let mut storage = ArchetypeStorage::new(build_archetype![i32, String, f64, bool], 256);

        assert_eq!(ArchetypeRef::new(0, 0), storage.add(EntityId::new(1)));
        assert_eq!(ArchetypeRef::new(0, 1), storage.add(EntityId::new(2)));
        assert_eq!(ArchetypeRef::new(0, 2), storage.add(EntityId::new(3)));

        storage.remove(&ArchetypeRef::new(0, 0));
        storage.remove(&ArchetypeRef::new(0, 0));
        storage.remove(&ArchetypeRef::new(0, 0));

        assert_eq!(ArchetypeRef::new(0, 0), storage.add(EntityId::new(4)));

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

        assert_eq!(ArchetypeRef::new(0, 0), src.add(e1));
        assert_eq!(ArchetypeRef::new(1, 0), src.add(e2));
        assert_eq!(ArchetypeRef::new(2, 0), src.add(e3));
        assert_eq!(ArchetypeRef::new(3, 0), src.add(e4));

        assert_eq!(
            (ArchetypeRef::new(0, 0), None),
            src.move_to(&mut dest, &ArchetypeRef::new(0, 0), 1i32)
                .unwrap()
        );
        assert_eq!(
            (ArchetypeRef::new(1, 0), None),
            src.move_to(&mut dest, &ArchetypeRef::new(1, 0), 2i32)
                .unwrap()
        );
        assert_eq!(
            (ArchetypeRef::new(2, 0), None),
            src.move_to(&mut dest, &ArchetypeRef::new(2, 0), 3i32)
                .unwrap()
        );
        assert_eq!(
            (ArchetypeRef::new(3, 0), None),
            src.move_to(&mut dest, &ArchetypeRef::new(3, 0), 4i32)
                .unwrap()
        );

        assert_eq!(0, src.row_count());
        assert_eq!(4, dest.row_count());

        // Check big chunks
        let mut src = ArchetypeStorage::new(build_archetype![String, f64, bool], 1000);
        let mut dest = ArchetypeStorage::new(build_archetype![String, f64, bool, i32], 1000);

        assert_eq!(ArchetypeRef::new(0, 0), src.add(e1));
        assert_eq!(ArchetypeRef::new(0, 1), src.add(e2));
        assert_eq!(ArchetypeRef::new(0, 2), src.add(e3));
        assert_eq!(ArchetypeRef::new(0, 3), src.add(e4));

        assert_eq!(
            (ArchetypeRef::new(0, 0), Some(e4)),
            src.move_to(&mut dest, &ArchetypeRef::new(0, 0), 1i32)
                .unwrap()
        );
        assert_eq!(
            (ArchetypeRef::new(0, 1), Some(e3)),
            src.move_to(&mut dest, &ArchetypeRef::new(0, 1), 2i32)
                .unwrap()
        );
        assert_eq!(
            (ArchetypeRef::new(0, 2), None),
            src.move_to(&mut dest, &ArchetypeRef::new(0, 1), 3i32)
                .unwrap()
        );
        assert_eq!(
            (ArchetypeRef::new(0, 3), None),
            src.move_to(&mut dest, &ArchetypeRef::new(0, 0), 4i32)
                .unwrap()
        );

        assert_eq!(0, src.row_count());
        assert_eq!(4, dest.row_count());
    }
}
