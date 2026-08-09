use std::{
    collections::{hash_map::Values, HashMap},
    sync::{
        atomic::{AtomicU32, Ordering},
        RwLock,
    },
};

use crate::{
    archetype::{Archetype, ArchetypeId, ArchetypeStorage},
    build_archetype,
    component::{cast, cast_mut, ComponentId},
    entity::{EntityId, EntityRef},
    error::EntityError,
    visitor::Visitor,
};

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
    pub(crate) fn new(chunk_size_in_bytes: usize) -> Self {
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

    pub(super) fn add_archetype(&mut self, archetype: Archetype) -> ArchetypeId {
        let arc_id = archetype.id;
        let arc_storage = ArchetypeStorage::new(archetype, self.chunk_size_in_bytes);
        self.archetypes.insert(arc_id, RwLock::new(arc_storage));
        arc_id
    }

    pub(super) fn add(&mut self, archetype: Option<ArchetypeId>) -> Result<EntityId, EntityError> {
        let arch_id = archetype.unwrap_or(self.def_arch_id);
        let seq = self.entity_seq.fetch_add(1, Ordering::Relaxed);
        let ent_id = EntityId::new(seq);
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

    pub(super) fn get<T, F, R>(&self, entity: EntityId, consumer: F) -> Option<R>
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

    pub(super) fn set<T>(&mut self, entity: EntityId, value: T) -> Result<(), EntityError>
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

    pub(super) fn remove(&mut self, entity: EntityId) -> Result<(), EntityError> {
        // Remove entity reference
        let ent_ref = self.entities.remove(&entity).ok_or(EntityError::NotFound)?;
        let storage = self
            .archetypes
            .get(&ent_ref.archetype)
            .ok_or(EntityError::NoSuchArchetype)?;
        // Remove entity's row from storage
        if let Some(swapped_ent_id) = storage.read().unwrap().remove(&ent_ref.arch_ref) {
            // Fix swapped entity reference
            self.entities.insert(swapped_ent_id, ent_ref);
        }
        Ok(())
    }

    pub(super) fn visit<V>(&self, mut visitor: V) -> (usize, usize, usize)
    where
        V: Visitor,
    {
        let mut arch_count = 0usize;
        let mut chunk_count = 0usize;
        let mut row_count = 0usize;
        for v in self.archetypes.values() {
            let guard = v.read().unwrap();
            if !visitor
                .columns()
                .iter()
                .all(|c| guard.archetype.has_component(c))
            {
                continue;
            }
            arch_count += 1;
            for chunk in guard.iter() {
                chunk_count += 1;
                row_count += visitor.visit(chunk);
            }
        }
        (arch_count, chunk_count, row_count)
    }

    pub(super) fn clear(&mut self) {
        self.entities.clear();
        for (_, lock) in self.archetypes.iter() {
            lock.write().unwrap().clear();
        }
    }

    pub(super) fn archetypes(&self) -> Values<'_, ArchetypeId, RwLock<ArchetypeStorage>> {
        self.archetypes.values()
    }
}
