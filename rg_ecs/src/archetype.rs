use std::{
    borrow::BorrowMut,
    collections::HashSet,
    hash::{DefaultHasher, Hash, Hasher},
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
/// ArchetypeStorage
///
/*
pub(crate) struct ArchetypeStorage {
    data: Vec<Box<dyn ComponentStorage>>,
}

impl ArchetypeStorage {
    pub fn new() -> ArchetypeStorage {
        let entity_id_column = TypedComponentStorage::<EntityId>::new(None);
        ArchetypeStorage {
            data: vec![Box::new(entity_id_column)],
        }
    }

    pub fn components(&self) -> HashSet<ComponentId> {
        self.data.iter().map(|v| v.id()).collect()
    }

    pub(crate) fn get<T: Default + 'static>(&self) -> Option<&TypedComponentStorage<T>> {
        let c = ComponentId::new::<T>();
        let storage = self.data.iter().find(|v| c == v.id())?;
        try_cast::<T>(storage)
    }

    pub(crate) fn get_mut<T: Default + 'static>(
        &mut self,
    ) -> Option<&mut TypedComponentStorage<T>> {
        let c = ComponentId::new::<T>();
        let storage = self.data.iter_mut().find(|v| c == v.id())?;
        try_cast_mut::<T>(storage)
    }

    pub(crate) fn move_to<T: Default + 'static>(
        &mut self,
        dest: &mut ArchetypeStorage,
        index: usize,
        value: T,
    ) {
        for s in self.data.iter_mut() {
            for d in dest.data.iter_mut() {
                if s.move_to(index, d.as_mut()) {
                    break;
                }
            }
        }
        for d in dest.data.iter_mut() {
            if let Some(c) = d.as_mut_any().downcast_mut::<TypedComponentStorage<T>>() {
                c.push(value);
                break;
            }
        }
        //unimplemented!()
    }

    pub(crate) fn extend_new<T: Default + 'static>(&self) -> ArchetypeStorage {
        let new_comp_storage = TypedComponentStorage::<T>::new();
        let mut data: Vec<Box<dyn ComponentStorage>> =
            self.data.iter().map(|v| v.create_new()).collect();
        data.push(Box::new(new_comp_storage));
        ArchetypeStorage { data }
    }

    pub(crate) fn add_row(&mut self) -> usize {
        let mut index = 0;
        for column in self.data.iter_mut() {
            index = column.push();
        }
        index
    }
}
*/
///
/// Tests
///
#[cfg(test)]
mod test {

    #[test]
    fn test() {
        // let storage = ArchetypeStorage::new();
        // let storage = storage.extend_new::<i32>();
        // let storage = storage.extend_new::<String>();
        // let mut storage = storage.extend_new::<f64>();

        // let c1 = storage.get::<i32>().unwrap();
        // let c3 = storage.get::<f64>().unwrap();

        // assert_eq!(0, storage.add_row());
        // assert_eq!(1, storage.add_row());
    }
}
