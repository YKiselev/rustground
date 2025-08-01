use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{
    archetype::{Archetype, ArchetypeId},
    entity::{EntityId, EntityStorage},
    error::EntityError,
    visitor::AsVisitor,
};

pub struct Entities {
    storage: RwLock<EntityStorage>,
}

impl Entities {

    /// Creates new instance
    pub fn new(chunk_size_in_bytes: usize) -> Self {
        Entities {
            storage: RwLock::new(EntityStorage::new(chunk_size_in_bytes)),
        }
    }

    /// Adds new archetype to this storage
    #[inline]
    pub fn add_archetype(&self, archetype: Archetype) -> ArchetypeId {
        self.write().add_archetype(archetype)
    }

    /// Adds new entity into this storage
    #[inline]
    pub fn add(&self, archetype: Option<ArchetypeId>) -> Result<EntityId, EntityError> {
        self.write().add(archetype)
    }

    /// Sets component on specified entity.
    /// Entity will be moved from one table to another (possibly new one) if current table doesn't have such component column.
    #[inline]
    pub fn set<T>(&self, entity: EntityId, value: T) -> Result<(), EntityError>
    where
        T: Default + 'static,
    {
        self.write().set(entity, value)
    }

    /// Gets the value of component of specified entity.
    #[inline]
    pub fn get<T, F, R>(&self, entity: EntityId, consumer: F) -> Option<R>
    where
        T: Default + 'static,
        R: 'static,
        F: FnOnce(Option<&T>) -> R,
    {
        self.read().get(entity, consumer)
    }

    /// Removes entity from storage
    #[inline]
    pub fn remove(&self, entity: EntityId) -> Result<(), EntityError> {
        self.write().remove(entity)
    }

    pub fn visit<F, Args>(&self, visitor: F)
    where
        F: AsVisitor<Args>,
    {
        self.read().visit(visitor.as_visitor())
    }

    /// Removes all entities from storage
    pub fn clear(&self) {
        self.write().clear();
    }

    #[inline]
    fn read(&self) -> RwLockReadGuard<'_, EntityStorage> {
        self.storage.read().unwrap()
    }

    #[inline]
    fn write(&self) -> RwLockWriteGuard<'_, EntityStorage> {
        self.storage.write().unwrap()
    }
}

///
/// Tests
///
#[cfg(test)]
mod test {

    use crate::{build_archetype, error::EntityError};

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

        entities.visit(|a: &i32, b: &f64| {
            println!("Got {a}, {b}");
        });
    }

    #[test]
    fn visit() -> Result<(), EntityError> {
        let entities = Entities::new(100);
        let arch_id1 = entities.add_archetype(build_archetype! {i32, f64, String, bool});
        for i in 0..100 {
            let id = entities.add(Some(arch_id1))?;
            entities.set(id, 5i32)?;
            entities.set(id, 3.14f64)?;
            entities.set(id, "Test".to_owned())?;
            entities.set(id, true)?;
        }

        entities.visit(|a: &i32, b: &f64, c: &String, d: &bool| {
            assert_eq!(5i32, *a);
            assert_eq!(3.14f64, *b);
            assert_eq!("Test", c);
            assert_eq!(true, *d);
        });
        Ok(())
    }
}
