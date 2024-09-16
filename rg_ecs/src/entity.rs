use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering}, RwLock,
    },
};


use crate::{
    archetype::{ArchetypeId, ArchetypeStorage, ARCH_ID_EMPTY},
    component::{
        cast, cast_mut, ComponentId, ComponentStorage,
    },
    error::EntityError,
};

///
/// EntityId
///
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct EntityId(pub(crate) usize);

///
/// EntityRef
///
#[derive(Clone, Copy, PartialEq, Eq)]
struct EntityRef {
    archetype: ArchetypeId,
    index: usize,
}

///
/// Entity storage
///
type EntityRefMap = HashMap<EntityId, EntityRef>;
type ArchetypeMap = HashMap<ArchetypeId, Box<ArchetypeStorage>>;

#[derive(Default)]
struct EntityStorage {
    entity_seq: AtomicUsize,
    entities: EntityRefMap,
    archetypes: ArchetypeMap,
}

impl EntityStorage {
    fn new() -> Self {
        let mut archetypes = HashMap::new();
        archetypes.insert(ARCH_ID_EMPTY, Box::new(ArchetypeStorage::default()));
        EntityStorage {
            entity_seq: AtomicUsize::new(0),
            entities: HashMap::new(),
            archetypes,
        }
    }

    fn add(&mut self, archetype: ArchetypeId) -> Result<EntityId, EntityError> {
        let seq = self.entity_seq.fetch_add(1, Ordering::Relaxed);
        let ent_id = EntityId(seq);
        let storage = self
            .archetypes
            .get(&ARCH_ID_EMPTY)
            .ok_or(EntityError::NotFound)?;
        let index = storage.add(ent_id);
        let ent_ref = EntityRef {
            archetype: ARCH_ID_EMPTY,
            index,
        };
        self.entities.insert(ent_id, ent_ref);
        Ok(ent_id)
    }

    fn get<T, F, R>(&self, entity: EntityId, consumer: F) -> Option<R>
    where
        T: Default + 'static,
        R: Sized + 'static,
        F: FnOnce(Option<&T>) -> R,
    {
        let e_ref = self.entities.get(&entity)?;
        let storage = self.archetypes.get(&e_ref.archetype)?;
        let column = storage.get(ComponentId::new::<T>())?;
        let guard = column.read().unwrap();
        Some(consumer(cast::<T>(guard.as_ref()).get(e_ref.index)))
    }

    fn set<T>(&mut self, entity: EntityId, value: T) -> Result<(), EntityError>
    where
        T: Default + 'static,
    {
        let comp_id = ComponentId::new::<T>();
        let EntityRef {
            archetype: base_archetype,
            index: base_index,
        } = self
            .entities
            .get(&entity)
            .ok_or_else(|| EntityError::NotFound)?;
        let base = self
            .archetypes
            .get(base_archetype)
            .ok_or_else(|| EntityError::NotFound)?;
        if let Some(column) = base.get(comp_id) {
            let mut guard = column.write()?;
            cast_mut::<T>(guard.as_mut()).set(*base_index, value);
        } else {
            let mut comps = base.components();
            comps.insert(comp_id);
            let dest_arc_id = ArchetypeId::new(comps.iter());
            if !self.archetypes.contains_key(&dest_arc_id) {
                let d = Box::new(base.new_extended::<T>());
                self.archetypes.insert(dest_arc_id, d);
            }
            let dest = self.archetypes.get(&dest_arc_id).unwrap();
            let base = self.archetypes.get(base_archetype).unwrap();
            let new_index = base.move_to(dest, *base_index, value);
            let new_ref = EntityRef {
                archetype: dest_arc_id,
                index: new_index,
            };
            self.entities.insert(entity, new_ref);
        }
        Ok(())
    }

    fn remove(&mut self, entity: EntityId) -> Result<(), EntityError> {
        let EntityRef {
            archetype,
            index,
        } = self
            .entities
            .remove(&entity)
            .ok_or_else(|| EntityError::NotFound)?;
        self.archetypes
            .remove(&archetype)
            .map(|_| ())
            .ok_or_else(|| EntityError::NotFound)
    }
}

///
/// Entities
///
#[derive(Default)]
pub struct Entities {
    storage: RwLock<EntityStorage>,
}

impl Entities {
    ///
    /// Creates new instance
    ///
    pub fn new() -> Self {
        Entities {
            storage: RwLock::new(EntityStorage::new()),
        }
    }

    ///
    /// Adds new entity into this storage
    ///
    pub fn add(&self, archetype: Option<ArchetypeId>) -> Result<EntityId, EntityError> {
        self.storage
            .write()
            .unwrap()
            .add(archetype.unwrap_or(ARCH_ID_EMPTY))
    }

    ///
    /// Sets component on specified entity.
    /// Entity will be moved from one table to another (possibly new one) if current table doesn't have such component column.
    ///
    pub fn set<T>(&self, entity: EntityId, value: T) -> Result<(), EntityError>
    where
        T: Default + 'static,
    {
        self.storage.write().unwrap().set(entity, value)
    }

    ///
    /// Gets the value of component of specified entity.
    ///
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
    pub fn remove(&self, entity: EntityId) -> Result<(), EntityError> {
        self.storage.write().unwrap().remove(entity)
    }

    // pub fn query1<T>(&self) -> impl Iterator<Item = &T>
    // where
    //     T: Default + 'static,
    // {
    //     let comp_id = ComponentId::new::<T>();
    //     self.archetypes
    //         .iter()
    //         .map(move |(_, v)| v.get(comp_id))
    //         .filter(|v| v.is_some())
    //         .map(|v| try_cast::<T>(v.unwrap()).unwrap())
    //         .flat_map(|v| v.iter())
    // }

    // pub fn query2_mut<T1, T2>(&mut self) -> impl Iterator<Item = (&mut T1, &mut T2)>
    // where
    //     T1: Default + 'static,
    //     T2: Default + 'static,
    // {
    //     let comp_id1 = ComponentId::new::<T1>();
    //     let comp_id2 = ComponentId::new::<T2>();
    //     self.archetypes
    //         .iter_mut()
    //         .map(move |(_, v)| (v.get_mut(comp_id1), v.get_mut(comp_id2)))
    //         .filter(|(v1, v2)| v1.is_some() && v2.is_some())
    //         .map(|(v1, v2)| (cast_mut::<T1>(v1.unwrap()), cast_mut::<T2>(v1.unwrap())))
    //         .flat_map(|(v1, v2)| izip!(v1.iter_mut(), v2.iter_mut()))
    // }
}

///
/// Tests
///
#[cfg(test)]
mod test {
    

    use super::Entities;

    #[test]
    fn test() {
        let entities = Entities::new();

        let e1 = entities.add(None).unwrap();
        let e2 = entities.add(None).unwrap();

        entities.set::<i32>(e1, 123).unwrap();
        entities.set::<f64>(e1, 3.14).unwrap();
        entities.set::<String>(e1, "test".to_owned()).unwrap();

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

        // assert_eq!(
        //     vec!["test", "yep yep yep"],
        //     entities.query1::<String>().collect::<Vec<_>>()
        // );
    }
}
