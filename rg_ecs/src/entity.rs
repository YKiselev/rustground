use std::{
    collections::{hash_map::Values, HashMap, HashSet},
    fmt::Debug,
    sync::{
        atomic::{AtomicU32, Ordering},
        RwLock, RwLockReadGuard,
    },
};

use crate::{
    archetype::{Archetype, ArchetypeId, ArchetypeRef, ArchetypeStorage, Chunk},
    build_archetype,
    component::{cast, cast_mut, ComponentId, ComponentStorage},
    error::EntityError,
};

///
/// EntityId
///
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
#[repr(transparent)]
pub struct EntityId(u32);

impl EntityId {
    pub fn new(id: u32) -> Self {
        EntityId(id)
    }
}

///
/// EntityRef
///
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntityRef {
    pub(crate) archetype: ArchetypeId,
    pub(crate) arch_ref: ArchetypeRef,
}

impl EntityRef {
    #[inline]
    fn new(archetype: ArchetypeId, arch_ref: ArchetypeRef) -> Self {
        Self {
            archetype,
            arch_ref,
        }
    }
}

///
/// Entity storage
///
type EntityRefMap = HashMap<EntityId, EntityRef>;
type ArchetypeMap = HashMap<ArchetypeId, RwLock<ArchetypeStorage>>;

pub(crate) struct EntityStorage {
    def_arch_id: ArchetypeId,
    chunk_size_in_bytes: usize,
    entity_seq: AtomicU32,
    entities: EntityRefMap,
    archetypes: ArchetypeMap,
}

impl EntityStorage {
    fn new(chunk_size_in_bytes: usize) -> Self {
        let mut archetypes = HashMap::new();
        let def_arc = build_archetype! {};
        let def_arch_id = def_arc.id;
        let def_storage = ArchetypeStorage::new(def_arc, chunk_size_in_bytes);
        archetypes.insert(def_arch_id, RwLock::new(def_storage));
        EntityStorage {
            def_arch_id,
            chunk_size_in_bytes,
            entity_seq: AtomicU32::new(0),
            entities: HashMap::with_capacity(chunk_size_in_bytes),
            archetypes,
        }
    }

    fn add_archetype(&mut self, archetype: Archetype) -> ArchetypeId {
        let arc_id = archetype.id;
        let arc_storage = ArchetypeStorage::new(archetype, self.chunk_size_in_bytes);
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
            .ok_or(EntityError::NoSuchArchetype)?
            .write()?;
        let arch_ref = storage.add(ent_id);
        let ent_ref = EntityRef {
            archetype: arch_id,
            arch_ref,
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
        let column = storage.get_at(ComponentId::new::<T>(), e_ref.arch_ref.chunk_index())?;
        let guard = column.read().unwrap();
        Some(consumer(
            cast::<T>(guard.as_ref()).get(e_ref.arch_ref.local_index()),
        ))
    }

    fn move_and_set<T>(
        &mut self,
        entity: EntityId,
        ent_ref: EntityRef,
        dest_arch: Archetype,
        value: T,
    ) -> Result<(), EntityError>
    where
        T: Default + 'static,
    {
        let dest_arch_id = dest_arch.id;
        self.archetypes.entry(dest_arch_id).or_insert_with(|| {
            RwLock::new(ArchetypeStorage::new(dest_arch, self.chunk_size_in_bytes))
        });
        let mut dest = self.archetypes[&dest_arch_id].write()?;
        let base = self.archetypes[&ent_ref.archetype].read()?;
        let (arch_ref, swapped_ent_id) = base.move_to(&mut dest, &ent_ref.arch_ref, value)?;
        self.entities
            .insert(entity, EntityRef::new(dest_arch_id, arch_ref));
        // If moved entity was swapped in source storage, fix it's ref
        if let Some(swapped_ent_id) = swapped_ent_id {
            self.entities.insert(swapped_ent_id, ent_ref);
        }
        Ok(())
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
        if let Some(column) = base.get_at(comp_id, ent_ref.arch_ref.chunk_index()) {
            let mut guard = column.write()?;
            cast_mut::<T>(guard.as_mut())[ent_ref.arch_ref.local_index()] = value;
            Ok(())
        } else {
            let dest_arch = base.archetype.to_builder().add::<T>().build();
            drop(base);
            self.move_and_set(entity, ent_ref, dest_arch, value)
        }
    }

    fn remove(&mut self, entity: EntityId) -> Result<(), EntityError> {
        // Remove entity reference
        let ent_ref = self.entities.remove(&entity).ok_or(EntityError::NotFound)?;
        let storage = self
            .archetypes
            .get(&ent_ref.archetype)
            .ok_or(EntityError::NoSuchArchetype)?;
        // Remove entitie's row from storage
        if let Some(swapped_ent_id) = storage.read().unwrap().remove(&ent_ref.arch_ref) {
            // Fix swapped entity reference
            self.entities.insert(swapped_ent_id, ent_ref);
        }
        Ok(())
    }

    fn visit<H>(&self, columns: &HashSet<ComponentId>, handler: H) -> (usize, usize, usize)
    where
        H: Fn(&Chunk) -> usize,
    {
        let mut arch_count: usize = 0;
        let mut chunk_count: usize = 0;
        let mut row_count: usize = 0;
        for (_, v) in self.archetypes.iter() {
            let guard = v.read().unwrap();
            if !columns.iter().all(|c| guard.archetype.has_component(c)) {
                continue;
            }
            for chunk in guard.iter() {
                row_count += (handler)(chunk);
                chunk_count += 1;
            }
            arch_count += 1;
        }
        (arch_count, chunk_count, row_count)
    }

    fn clear(&mut self) {
        self.entities.clear();
        for (_, lock) in self.archetypes.iter() {
            lock.write().unwrap().clear();
        }
    }

    pub(crate) fn archetypes(&self) -> Values<'_, ArchetypeId, RwLock<ArchetypeStorage>> {
        self.archetypes.values()
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

    pub fn visit<H>(&self, columns: &HashSet<ComponentId>, handler: H) -> (usize, usize, usize)
    where
        H: Fn(&Chunk) -> usize,
    {
        self.storage.read().unwrap().visit(columns, handler)
    }

    ///
    /// Removes all entities from storage
    ///
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

    use crate::{build_archetype, component::ComponentId, entity::EntityId, visitor::visit_2};

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

        let columns = HashSet::from([ComponentId::new::<EntityId>(), ComponentId::new::<String>()]);
        let v2 = visit_2::<EntityId, String, _>(move |(_, _)| {});
        let (ac, cc, rc) = entities.visit(&columns, v2);
        println!("archs={}, chunks={}, rows={}", ac, cc, rc);
    }
}
