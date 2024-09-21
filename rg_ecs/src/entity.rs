use std::{
    collections::HashMap, fmt::Debug, sync::{
        atomic::{AtomicUsize, Ordering},
        RwLock,
    }
};

use itertools::izip;

use crate::{
    archetype::{Archetype, ArchetypeBuilder, ArchetypeId, ArchetypeStorage},
    build_archetype,
    component::{cast, cast_mut, ComponentId, ComponentStorage},
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
type ArchetypeMap = HashMap<ArchetypeId, RwLock<ArchetypeStorage>>;

struct EntityStorage {
    def_arch_id: ArchetypeId,
    chunk_size: usize,
    entity_seq: AtomicUsize,
    entities: EntityRefMap,
    archetypes: ArchetypeMap,
}

impl EntityStorage {
    fn new(chunk_size: usize) -> Self {
        let mut archetypes = HashMap::new();
        let def_arc = build_archetype! {};
        let def_arch_id = def_arc.id;
        let def_storage = ArchetypeStorage::new(def_arc, chunk_size);
        archetypes.insert(def_arch_id, RwLock::new(def_storage));
        EntityStorage {
            def_arch_id,
            chunk_size,
            entity_seq: AtomicUsize::new(0),
            entities: HashMap::new(),
            archetypes,
        }
    }

    fn add_archetype(&mut self, archetype: Archetype) -> ArchetypeId {
        let arc_id = archetype.id;
        let arc_storage = ArchetypeStorage::new(archetype, self.chunk_size);
        self.archetypes.insert(arc_id, RwLock::new(arc_storage));
        arc_id
    }

    fn add(&mut self, archetype: Option<ArchetypeId>) -> Result<EntityId, EntityError> {
        let arch_id = archetype.unwrap_or(self.def_arch_id);
        let seq = self.entity_seq.fetch_add(1, Ordering::Relaxed);
        let ent_id = EntityId(seq);
        let mut storage = self
            .archetypes
            .get(&arch_id)
            .ok_or(EntityError::NotSuchArchetype)?
            .write()?;
        let index = storage.add(ent_id);
        let ent_ref = EntityRef {
            archetype: arch_id,
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
        let storage = self.archetypes.get(&e_ref.archetype)?.read().ok()?;
        let (column, local_idx) = storage.get_at(ComponentId::new::<T>(), e_ref.index)?;
        let guard = column.read().unwrap();
        Some(consumer(cast::<T>(guard.as_ref()).get(local_idx)))
    }

    fn set<T>(&mut self, entity: EntityId, value: T) -> Result<(), EntityError>
    where
        T: Default + 'static,
    {
        let comp_id = ComponentId::new::<T>();
        let ent_ref = self
            .entities
            .get(&entity)
            .ok_or_else(|| EntityError::NotFound)?
            .clone();
        let base = self
            .archetypes
            .get(&ent_ref.archetype)
            .ok_or_else(|| EntityError::NotFound)?
            .read()?;
        if let Some((column, local_index)) = base.get_at(comp_id, ent_ref.index) {
            let mut guard = column.write()?;
            cast_mut::<T>(guard.as_mut()).set(local_index, value);
        } else {
            let dest_arch = base.archetype.to_builder().add::<T>().build();
            let dest_arch_id = dest_arch.id;
            drop(base);
            if !self.archetypes.contains_key(&dest_arch_id) {
                let dest = ArchetypeStorage::new(dest_arch, self.chunk_size);
                self.archetypes.insert(dest_arch_id, RwLock::new(dest));
            }
            let mut dest = self.archetypes[&dest_arch_id].write().unwrap();
            let base = self.archetypes[&ent_ref.archetype].read().unwrap();
            let (new_index, moved_ent_id) = base.move_to(&mut dest, ent_ref.index, value);
            self.entities.insert(
                entity,
                EntityRef {
                    archetype: dest_arch_id,
                    index: new_index,
                },
            );
            if let Some(moved_ent_id) = moved_ent_id {
                self.entities.insert(moved_ent_id, ent_ref);
            }
        }
        Ok(())
    }

    fn remove(&mut self, entity: EntityId) -> Result<(), EntityError> {
        let EntityRef { archetype, index } =
            self.entities.remove(&entity).ok_or(EntityError::NotFound)?;
        let storage = self
            .archetypes
            .get(&archetype)
            .ok_or(EntityError::NotSuchArchetype)?;
        if let Some(moved_ent_id) = storage.read().unwrap().remove(index) {
            // Entity was removed with swap remove, so it's place now occupied by another entity (moved)
            // Let's update it's reference
            self.entities
                .insert(moved_ent_id, EntityRef { archetype, index });
        }
        Ok(())
    }

    fn query2_mut<T1, T2>(&self)
    // -> impl Iterator<Item = (&mut T1, &mut T2)>
    where
        T1: Default + 'static + Debug,
        T2: Default + 'static + Debug,
    {
        let comp_id1 = ComponentId::new::<T1>();
        let comp_id2 = ComponentId::new::<T2>();
        for (_, v) in self.archetypes.iter() {
            let guard = v.read().unwrap();
            for chunk in guard.iter() {
                let lock1 = chunk.get_column(comp_id1);
                let lock2 = chunk.get_column(comp_id2);
                if lock1.is_none() || lock2.is_none() {
                    break;
                }
                let mut guard1 = lock1.unwrap().write().unwrap();
                let mut guard2 = lock2.unwrap().write().unwrap();
                let iter1 = cast_mut::<T1>(guard1.as_mut()).iter();
                let iter2 = cast_mut::<T2>(guard2.as_mut()).iter();
                let iter = izip!(iter1, iter2);
                for v in iter {
                    dbg!(v);
                }
            }
        }
    }
}

///
/// Entities
///
pub struct Entities {
    storage: RwLock<EntityStorage>,
}

impl Entities {
    ///
    /// Creates new instance
    ///
    pub fn new() -> Self {
        Entities {
            storage: RwLock::new(EntityStorage::new(256)),
        }
    }

    ///
    /// Adds new archetype to this storage
    ///
    pub fn add_archetype(&self, archetype: Archetype) -> ArchetypeId {
        self.storage.write().unwrap().add_archetype(archetype)
    }

    ///
    /// Adds new entity into this storage
    ///
    pub fn add(&self, archetype: Option<ArchetypeId>) -> Result<EntityId, EntityError> {
        self.storage.write().unwrap().add(archetype)
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

    pub fn query2_mut<T1, T2>(&self)
    // -> impl Iterator<Item = (&mut T1, &mut T2)>
    where
        T1: Default + 'static + Debug,
        T2: Default + 'static + Debug,
    {
        self.storage.read().unwrap().query2_mut::<T1, T2>();
    }
}

///
/// Tests
///
#[cfg(test)]
mod test {

    use crate::{
        archetype::{self, ArchetypeBuilder},
        build_archetype,
    };

    use super::Entities;

    #[test]
    fn test() {
        let entities = Entities::new();

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

        entities.query2_mut::<i32, String>();
        // assert_eq!(
        //     vec!["test", "yep yep yep"],
        //     entities.query1::<String>().collect::<Vec<_>>()
        // );
    }
}
