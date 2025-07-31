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
    chunk::Chunk, component::{cast, cast_mut, ComponentId, ComponentStorage, TypedComponentStorage}, entity::EntityId, error::EntityError
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
/// ColumnFactory
///
trait ColumnFactory {
    fn create(&self, capacity: usize) -> Box<dyn ComponentStorage + 'static>;
    fn item_size(&self) -> usize;
}

#[derive(Default)]
struct TypedColumnFactory<T>(PhantomData<T>)
where
    T: Default + 'static;

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
pub struct ArchetypeBuilder(HashMap<ComponentId, Arc<dyn ColumnFactory>>);

impl ArchetypeBuilder {
    pub fn new() -> Self {
        ArchetypeBuilder(HashMap::with_capacity(4)).add::<EntityId>()
    }

    pub fn add<T: Default + 'static>(mut self) -> Self {
        let comp_id = ComponentId::new::<T>();
        self.0
            .insert(comp_id, Arc::new(TypedColumnFactory::<T>::default()));
        self
    }

    pub fn build(self) -> Archetype {
        let mut hasher = FxHasher32::default();
        for id in self.0.keys().sorted() {
            id.hash(&mut hasher);
        }
        Archetype {
            id: ArchetypeId(hasher.finish() as u32),
            factories: self.0,
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
    pub(crate) fn new_chunk(&self, capacity: usize) -> Chunk {
        Chunk::new(
            self.factories
                .iter()
                .map(|(id, f)| (*id, RwLock::new(f.create(capacity))))
                .collect(),
            capacity,
        )
    }

    pub fn to_builder(&self) -> ArchetypeBuilder {
        ArchetypeBuilder(
            self.factories
                .iter()
                .map(|(id, f)| (*id, Arc::clone(f)))
                .collect(),
        )
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


#[cfg(test)]
mod test {

    use crate::{
        archetype_storage::{ArchetypeStorage, StorageRowRef}, entity::EntityId
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
