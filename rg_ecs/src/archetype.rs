use std::{
    collections::{HashMap, HashSet},
    hash::{DefaultHasher, Hash, Hasher},
    sync::RwLock,
};

use once_cell::sync::{self, Lazy};

use crate::{
    component::{cast_mut, ComponentId, ComponentStorage, TypedComponentStorage},
    entity::EntityId,
};
///
/// Constants
///
pub(crate) static COLUMN_ENTITY_ID: Lazy<ComponentId> = sync::Lazy::new(|| ComponentId::new::<EntityId>());
pub(crate) const ARCH_ID_EMPTY: ArchetypeId = ArchetypeId(0);

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
///
/// Archetype builder
///
pub struct ArchetypeBuilder {
    factories: HashMap<ComponentId, Box<dyn Fn() -> Box<dyn ComponentStorage + 'static>>>,
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
            Box::new(|| Box::new(TypedComponentStorage::<T>::new())),
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

pub struct Archetype {
    pub id: ArchetypeId,
    factories: HashMap<ComponentId, Box<dyn Fn() -> Box<dyn ComponentStorage + 'static>>>,
}

impl Archetype {
    fn create_storage(&self) -> (ArchetypeId, ArchetypeStorage) {
        (
            self.id,
            ArchetypeStorage {
                columns: self
                    .factories
                    .iter()
                    .map(|(id, f)| (*id, RwLock::new((f)())))
                    .collect(),
            },
        )
    }
}

///
/// ArchetypeStorage
///
type ColumnMap = HashMap<ComponentId, RwLock<Box<dyn ComponentStorage>>>;

#[derive(Default)]
pub(crate) struct ArchetypeStorage {
    columns: ColumnMap,
}

impl ArchetypeStorage {
    // pub fn new(columns: HashSet<ComponentId>) -> ArchetypeStorage {
    //     let entity_id_column = TypedComponentStorage::<EntityId>::new(None);
    //     ArchetypeStorage {
    //         data: vec![Box::new(entity_id_column)],
    //     }
    // }

    pub(crate) fn get_by_type<T>(&self) -> Option<&RwLock<Box<dyn ComponentStorage>>>
    where
        T: Default + 'static,
    {
        self.get(ComponentId::new::<T>())
    }

    pub(crate) fn get(&self, comp_id: ComponentId) -> Option<&RwLock<Box<dyn ComponentStorage>>> {
        self.columns.get(&comp_id)
    }

    pub(crate) fn move_to<T: Default + 'static>(
        &self,
        dest: &ArchetypeStorage,
        index: usize,
        value: T,
    ) -> usize {
        for (comp_id, column) in self.columns.iter() {
            column
                .write()
                .unwrap()
                .move_to(index, dest.get(*comp_id).unwrap().write().unwrap().as_mut());
        }
        cast_mut::<T>(dest.get_by_type::<T>().unwrap().write().unwrap().as_mut())
                .push(value)
    }

    pub(crate) fn new_extended<T: Default + 'static>(&self) -> ArchetypeStorage {
        let new_comp_id = ComponentId::new::<T>();
        let new_comp_storage = TypedComponentStorage::<T>::new();
        let mut columns: ColumnMap = self
            .columns
            .iter()
            .map(|(k, v)| (*k, RwLock::new(v.read().unwrap().create_new())))
            .collect();
        columns.insert(new_comp_id, RwLock::new(Box::new(new_comp_storage)));
        ArchetypeStorage { columns }
    }

    pub(crate) fn add(&self, ent_id: EntityId) -> usize {
        let mut index = 0;
        for (id, column) in self.columns.iter() {
            if *id == *COLUMN_ENTITY_ID {
                column.write().unwrap().add();
            } else {
                index = column.write().unwrap().add();
            }
        }
        index
    }

    pub(crate) fn remove(&self, index: usize) {
        for column in self.columns.values() {
            column.write().unwrap().remove(index);
        }
    }

    pub(crate) fn components(&self) -> HashSet<ComponentId> {
        self.columns.keys().cloned().collect()
    }
}

///
/// Macros
///
macro_rules! archetype {
    ($($column_type:ty),+) => {
        $crate::archetype::ArchetypeBuilder::new()
        $(.add::<$column_type>())*
        .build()
    };
}

///
/// Tests
///
#[cfg(test)]
mod test {
    use crate::{
        component::ComponentStorage,
        entity::EntityId,
    };

    

    #[test]
    fn test() {
        let archetype = archetype![i32, String, f64, bool];
        let (id, storage) = archetype.create_storage();

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
