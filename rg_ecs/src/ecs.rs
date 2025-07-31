use std::{
    collections::HashSet,
    sync::{RwLock, RwLockReadGuard},
};

use crate::{
    archetype::{Archetype, ArchetypeId},
    chunk::Chunk,
    component::ComponentId,
    entity::EntityId,
    entity_storage::EntityStorage,
    error::EntityError, visitor::{AsVisitor, Visitor},
};

pub struct Entities {
    storage: RwLock<EntityStorage>,
}

impl Entities {
    ///
    /// Creates new instance
    ///
    pub fn new(chunk_size_in_bytes: usize) -> Self {
        Entities {
            storage: RwLock::new(EntityStorage::new(chunk_size_in_bytes)),
        }
    }

    ///
    /// Adds new archetype to this storage
    ///
    #[inline]
    pub fn add_archetype(&self, archetype: Archetype) -> ArchetypeId {
        self.storage.write().unwrap().add_archetype(archetype)
    }

    ///
    /// Adds new entity into this storage
    ///
    #[inline]
    pub fn add(&self, archetype: Option<ArchetypeId>) -> Result<EntityId, EntityError> {
        self.storage.write().unwrap().add(archetype)
    }

    ///
    /// Sets component on specified entity.
    /// Entity will be moved from one table to another (possibly new one) if current table doesn't have such component column.
    ///
    #[inline]
    pub fn set<T>(&self, entity: EntityId, value: T) -> Result<(), EntityError>
    where
        T: Default + 'static,
    {
        self.storage.write().unwrap().set(entity, value)
    }

    ///
    /// Gets the value of component of specified entity.
    ///
    #[inline]
    pub fn get<T, F, R>(&self, entity: EntityId, consumer: F) -> Option<R>
    where
        T: Default + 'static,
        R: 'static,
        F: FnOnce(Option<&T>) -> R,
    {
        self.storage.read().unwrap().get(entity, consumer)
    }

    ///
    /// Removes entity from storage
    ///
    #[inline]
    pub fn remove(&self, entity: EntityId) -> Result<(), EntityError> {
        self.storage.write().unwrap().remove(entity)
    }

    pub fn visit<F, Args>(&self, visitor: F) -> (usize, usize, usize)
    where
        F: AsVisitor<Args>,
    {
        self.storage.read().unwrap().visit(visitor.as_visitor())
    }

    /// Removes all entities from storage
    pub fn clear(&self) {
        self.storage.write().unwrap().clear();
    }

    #[doc(hidden)]
    pub(crate) fn read(&self) -> RwLockReadGuard<'_, EntityStorage> {
        self.storage.read().unwrap()
    }
}

///
/// Tests
///
#[cfg(test)]
mod test {

    use std::collections::HashSet;

    use crate::{build_archetype, component::ComponentId, entity::EntityId};

    use super::Entities;

    #[test]
    fn test() {
        let entities = Entities::new(100);

        let arch_id1 = entities.add_archetype(build_archetype! {i32, f64, String});

        let e1 = entities.add(None).unwrap();
        let e2 = entities.add(Some(arch_id1)).unwrap();
        let e3 = entities.add(Some(arch_id1)).unwrap();
        entities.set(e3, "hehe").unwrap();
        let e4 = entities.add(Some(arch_id1)).unwrap();
        let e5 = entities.add(Some(arch_id1)).unwrap();
        entities.remove(e3).unwrap();
        entities.remove(e5).unwrap();
        entities.remove(e4).unwrap();
        let e5 = entities.add(Some(arch_id1)).unwrap();

        entities.set::<i32>(e1, 123).unwrap();
        entities.set::<f64>(e1, 3.14).unwrap();
        entities.set(e1, "test".to_owned()).unwrap();

        entities.set::<i32>(e2, 456).unwrap();
        entities.set::<f64>(e2, 5.5).unwrap();
        entities
            .set::<String>(e2, "yep yep yep".to_owned())
            .unwrap();

        assert_eq!(123, entities.get::<i32, _, _>(e1, |v| *v.unwrap()).unwrap());
        assert_eq!(
            3.14,
            entities.get::<f64, _, _>(e1, |v| *v.unwrap()).unwrap()
        );
        assert_eq!(
            "test".to_owned(),
            entities
                .get::<String, _, _>(e1, |v| v.unwrap().clone())
                .unwrap()
        );

        assert_eq!(456, entities.get::<i32, _, _>(e2, |v| *v.unwrap()).unwrap());
        assert_eq!(5.5, entities.get::<f64, _, _>(e2, |v| *v.unwrap()).unwrap());
        assert_eq!(
            "yep yep yep".to_owned(),
            entities
                .get::<String, _, _>(e2, |v| v.unwrap().clone())
                .unwrap()
        );

        //let columns = HashSet::from([ComponentId::new::<EntityId>(), ComponentId::new::<String>()]);
        // let v2 = visit_2::<EntityId, String, _>(move |(_, _)| {});
        // let (ac, cc, rc) = entities.visit(&columns, v2);
        // println!("archs={}, chunks={}, rows={}", ac, cc, rc);
        entities.visit(|a:&i32, b: &f64|{
            println!("Got {a}, {b}");
        });
    }

     #[test]
    fn visit() {
        let entities = Entities::new(100);
        let arch_id1 = entities.add_archetype(build_archetype! {i32, f64, String, bool});
        for i in 0..100 {
            entities.add(Some(arch_id1)).unwrap();
        }

        entities.visit(|a:&i32, b: &f64|{
            println!("Got {a}, {b}");
        });
    }
}
