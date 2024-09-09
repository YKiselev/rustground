use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{
    archetype::{ArchetypeId, ArchetypeStorage, EMPTY_ARCHETYPE_ID},
    component::{try_cast, ComponentId},
};

///
/// EntityId
///
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct EntityId(usize);

///
/// EntityRef
///
#[derive(Clone, Copy, PartialEq, Eq)]
struct EntityRef {
    archetype: ArchetypeId,
    index: usize,
}

///
/// Entities
///
type EntityRefMap = HashMap<EntityId, EntityRef>;
type ArchetypeMap = HashMap<ArchetypeId, ArchetypeStorage>;
pub struct Entities {
    entity_seq: AtomicUsize,
    entities: EntityRefMap,
    archetypes: ArchetypeMap,
}

impl Entities {
    pub fn new() -> Self {
        let entities = HashMap::new();
        let mut archetypes = HashMap::new();
        archetypes.insert(EMPTY_ARCHETYPE_ID, ArchetypeStorage::new());
        Entities {
            entity_seq: AtomicUsize::new(1),
            entities,
            archetypes,
        }
    }

    pub fn add(&mut self) -> Result<EntityId, EntityError> {
        let seq = self.entity_seq.fetch_add(1, Ordering::AcqRel);
        let id = EntityId(seq);
        let storage = self
            .archetypes
            .get_mut(&EMPTY_ARCHETYPE_ID)
            .ok_or(EntityError::NotFound)?;
        let index = storage.add_row();
        self.entities.insert(
            id,
            EntityRef {
                archetype: EMPTY_ARCHETYPE_ID,
                index: 0,
            },
        );
        Ok(id)
    }

    pub fn set<T: Default + 'static>(
        &mut self,
        entity: EntityId,
        value: T,
    ) -> Result<(), EntityError> {
        let entity_ref = self
            .entities
            .get(&entity)
            .map(|v| v.clone())
            .ok_or_else(|| EntityError::NotFound)?;
        let base = self
            .archetypes
            .get_mut(&entity_ref.archetype)
            .ok_or_else(|| EntityError::NotFound)?;
        if let Some(column) = base.get_mut::<T>() {
            column.set(entity_ref.index, value);
        } else {
            let mut comps = base.components();
            comps.insert(ComponentId::new::<T>());
            let existing = self
                .archetypes
                .iter()
                .find(|(_, v)| v.components() == comps)
                .map(|(k, _)| k.clone());

            let (dest_id, mut dest) = if let Some(k) = existing {
                self.archetypes.remove_entry(&k).unwrap()
            } else {
                let new_storage = self
                    .archetypes
                    .get_mut(&entity_ref.archetype)
                    .unwrap()
                    .extend_new::<T>();
                let new_id = ArchetypeId::new(&comps);
                (new_id, new_storage)
            };
            let mut src = self.archetypes.get_mut(&entity_ref.archetype).unwrap();
            src.move_to(&mut dest, entity_ref.index, value);
            self.archetypes.insert(dest_id, dest);
        }
        Ok(())
    }
}

///
/// EntityError
///
#[derive(Debug)]
pub enum EntityError {
    NotFound,
}

impl Error for EntityError {}

impl Display for EntityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityError::NotFound => write!(f, "No such entity!"),
        }
    }
}

///
/// Tests
///
#[cfg(test)]
mod test {
    use std::sync::Mutex;

    use super::Entities;

    #[test]
    fn test() {
        let mut entities = Entities::new();

        let e1 = entities.add().unwrap();
        let e2 = entities.add().unwrap();

        entities.set::<i32>(e1, 123).unwrap();
        entities.set::<f64>(e1, 3.14).unwrap();
        //entities.set::<String>(e1, "test".to_owned()).unwrap();
    }
}
