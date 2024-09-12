use std::{
    borrow::BorrowMut,
    collections::{HashMap, HashSet},
    hash::{DefaultHasher, Hash, Hasher},
    sync::RwLock,
};

use crate::{
    component::{try_cast, try_cast_mut, ComponentId, ComponentStorage, TypedComponentStorage},
    entity::EntityId,
};

///
/// Constants
///
pub(crate) const EMPTY_ARCHETYPE_ID: ArchetypeId = ArchetypeId(0);

///
/// ArchetypeId
///
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct ArchetypeId(u64);

impl ArchetypeId {
    pub fn new(components: &HashSet<ComponentId>) -> Self {
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
            Box::new(|| Box::new(TypedComponentStorage::<T>::new(None))),
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
pub(crate) struct ArchetypeStorage {
    columns: HashMap<ComponentId, RwLock<Box<dyn ComponentStorage>>>,
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

    // pub(crate) fn move_to<T: Default + 'static>(
    //     &mut self,
    //     dest: &mut ArchetypeStorage,
    //     index: usize,
    //     value: T,
    // ) {
    //     for s in self.data.iter_mut() {
    //         for d in dest.data.iter_mut() {
    //             if s.move_to(index, d.as_mut()) {
    //                 break;
    //             }
    //         }
    //     }
    //     for d in dest.data.iter_mut() {
    //         if let Some(c) = d.as_mut_any().downcast_mut::<TypedComponentStorage<T>>() {
    //             c.push(value);
    //             break;
    //         }
    //     }
    //     //unimplemented!()
    // }

    // pub(crate) fn extend_new<T: Default + 'static>(&self) -> ArchetypeStorage {
    //     let new_comp_storage = TypedComponentStorage::<T>::new();
    //     let mut data: Vec<Box<dyn ComponentStorage>> =
    //         self.data.iter().map(|v| v.create_new()).collect();
    //     data.push(Box::new(new_comp_storage));
    //     ArchetypeStorage { data }
    // }

    pub(crate) fn add(&self) -> usize {
        let mut index = 0;
        for column in self.columns.values() {
            index = column.write().unwrap().add();
        }
        index
    }

    pub(crate) fn remove(&self, index: usize) {
        for column in self.columns.values() {
            column.write().unwrap().remove(index);
        }
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
        component::{cast, ComponentStorage},
        entity::EntityId,
    };

    use super::ArchetypeBuilder;

    #[test]
    fn macros() {
        let archetype = archetype![i32, String, f64, bool];
    }

    #[test]
    fn test() {
        let archetype = archetype![i32, String, f64, bool];
        let (id, storage) = archetype.create_storage();

        assert_eq!(0, storage.add());
        assert_eq!(1, storage.add());
        assert_eq!(2, storage.add());

        storage.remove(0);
        storage.remove(1);
        storage.remove(0);

        assert_eq!(0, storage.add());

        storage.get_by_type::<i32>().unwrap();
        storage.get_by_type::<f64>().unwrap();
        storage.get_by_type::<String>().unwrap();
        storage.get_by_type::<bool>().unwrap();
        storage.get_by_type::<EntityId>().unwrap();

        assert!(storage.get_by_type::<i8>().is_none());
    }
}
