use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
};

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
            factories: HashMap::new(),
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
        for id in self.factories.keys() {
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
    pub(crate) fn new_chunk(&self, capacity: usize) -> Chunk {
        Chunk::new(
            self.factories
                .iter()
                .map(|(id, f)| (*id, RwLock::new((f)(capacity))))
                .collect(),
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

struct Chunk {
    columns: ColumnMap,
    available_rows: AtomicUsize,
}

impl Chunk {
    fn new(columns: ColumnMap) -> Self {
        Chunk {
            columns,
            available_rows: AtomicUsize::default(),
        }
    }

    fn available(&self) -> usize {
        self.available_rows.load(Ordering::Acquire)
    }
}

///
/// ArchetypeStorage
///
pub(crate) struct ArchetypeStorage {
    pub(crate) archetype: Archetype,
    chunk_size: usize,
    chunks: Vec<Box<Chunk>>,
}

impl ArchetypeStorage {
    pub(crate) fn new(archetype: Archetype, chunk_size: usize) -> Self {
        ArchetypeStorage {
            archetype,
            chunk_size,
            chunks: vec![],
        }
    }

    pub(crate) fn get_by_type<T>(&self) -> Option<&RwLock<Box<dyn ComponentStorage>>>
    where
        T: Default + 'static,
    {
        self.get(ComponentId::new::<T>())
    }

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
    pub(crate) fn get_chunk(&mut self) -> Option<&Box<Chunk>> {
        for chunk in self.chunks {
            if chunk.available() > 0 {
                return Some(&chunk);
            }
        }
        // No unfilled chunks (or no chunks at all). Let's add new
        let chunk = Box::new(self.archetype.new_chunk(self.chunk_size));
        let ch_ref = chunk.as_ref();
        self.chunks.push(chunk);
        self.chunks.last()
    }

    ///
    /// Returns column with at least 1 free row
    ///
    pub(crate) fn get(
        &mut self,
        comp_id: ComponentId,
    ) -> Option<&RwLock<Box<dyn ComponentStorage>>> {
        let chunk = self.get_chunk()?;
        chunk.columns.get(&comp_id)
    }

    pub(crate) fn get_at(
        &self,
        comp_id: ComponentId,
        index: usize,
    ) -> Option<(&RwLock<Box<dyn ComponentStorage>>, usize)> {
        let (chunk, local_index) = self.chunk(index)?;
        chunk.columns.get(&comp_id).map(|c| (c, local_index))
    }

    ///
    /// Returns chunk for entity index
    ///
    fn chunk(&self, index: usize) -> Option<(&Box<Chunk>, usize)> {
        let chunk_index = index / self.chunk_size;
        self.chunks
            .get(chunk_index)
            .map(|ch| (ch, index % self.chunk_size))
    }

    pub(crate) fn move_to<T: Default + 'static>(
        &self,
        dest: &mut ArchetypeStorage,
        index: usize,
        value: T,
    ) -> usize {
        let (chunk, local_index) = self.chunk(index).unwrap();
        for (comp_id, column) in chunk.columns.iter() {
            column.write().unwrap().move_to(
                local_index,
                dest.get(*comp_id).unwrap().write().unwrap().as_mut(),
            );
        }
        cast_mut::<T>(dest.get_by_type::<T>().unwrap().write().unwrap().as_mut()).push(value)
    }

    ///
    /// Creates new ArchetypeStorage from this one with additional column of type T
    ///
    pub(crate) fn new_extended<T: Default + 'static>(&self) -> ArchetypeStorage {
        let new_archetype = self.archetype.to_builder().add::<T>().build();
        ArchetypeStorage::new(new_archetype, self.chunk_size)
    }

    ///
    /// Adds new row for passed entity to this storage
    ///
    pub(crate) fn add(&mut self, ent_id: EntityId) -> usize {
        let chunk = self.get_chunk().unwrap();

        let mut index = 0;
        for (id, column) in chunk.columns.iter() {
            let mut guard = column.write().unwrap();
            if *id == *COLUMN_ENTITY_ID {
                index = cast_mut::<EntityId>(guard.as_mut()).push(ent_id);
            } else {
                index = column.write().unwrap().add();
            }
        }
        index
    }

    pub(crate) fn remove(&self, index: usize) {
        if let Some((chunk, local_index)) = self.chunk(index) {
            for column in chunk.columns.values() {
                column.write().unwrap().remove(local_index);
            }
        }
    }

    // pub(crate) fn components(&self) -> HashSet<ComponentId> {
    //     self.columns.keys().cloned().collect()
    // }
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
    use crate::{archetype::ArchetypeStorage, component::ComponentStorage, entity::EntityId};

    #[test]
    fn archetype_to_builder() {
        let archetype = build_archetype![i32, String, f64, bool];
        let builder = archetype.to_builder();
        let archetype2 = builder.build();
        assert_eq!(archetype, archetype2);
    }

    #[test]
    fn test() {
        //let archetype = build_archetype![i32, String, f64, bool];
        let storage = ArchetypeStorage::new(build_archetype![i32, String, f64, bool], 256);
        //let (id, storage) = archetype.create_storage();

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
    }
}
