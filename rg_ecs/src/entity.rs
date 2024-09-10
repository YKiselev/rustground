use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt::Display,
    slice::Iter,
    sync::atomic::{AtomicUsize, Ordering},
};

use itertools::izip;

use crate::{
    archetype::{ArchetypeId, EMPTY_ARCHETYPE_ID},
    component::{
        cast, cast_mut, try_cast, try_cast_mut, ComponentId, ComponentStorage,
        TypedComponentStorage,
    },
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
type ArchetypeMap = HashMap<ArchetypeId, Box<dyn ComponentStorage>>;

pub struct Entities {
    entity_seq: AtomicUsize,
    entities: EntityRefMap,
    archetypes: ArchetypeMap,
}

impl Entities {
    pub fn new() -> Self {
        let entities = HashMap::new();
        let mut archetypes: ArchetypeMap = HashMap::new();
        archetypes.insert(
            EMPTY_ARCHETYPE_ID,
            Box::new(TypedComponentStorage::<EntityId>::new(None)),
        );
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
        if let Some(s) = storage
            .get_mut(ComponentId::new::<EntityId>())
            .and_then(|v| {
                v.as_mut_any()
                    .downcast_mut::<TypedComponentStorage<EntityId>>()
            })
        {
            s.set(index, id);
        }
        self.entities.insert(
            id,
            EntityRef {
                archetype: EMPTY_ARCHETYPE_ID,
                index,
            },
        );
        Ok(id)
    }

    pub fn set<T>(&mut self, entity: EntityId, value: T) -> Result<(), EntityError>
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
            .get_mut(base_archetype)
            .ok_or_else(|| EntityError::NotFound)?;
        if let Some(column) = base.get_mut(comp_id) {
            try_cast_mut::<T>(column).unwrap().set(*base_index, value);
        } else {
            let mut comps = base.components();
            comps.insert(comp_id);
            let existing = self
                .archetypes
                .iter()
                .find(|(_, v)| v.components() == comps)
                .map(|(k, _)| k.clone());

            let (dest_id, mut dest) = if let Some(k) = existing {
                self.archetypes.remove_entry(&k).unwrap()
            } else {
                let base_columns = self
                    .archetypes
                    .get_mut(base_archetype)
                    .unwrap()
                    .create_new();
                let new_storage: Box<dyn ComponentStorage> =
                    Box::new(TypedComponentStorage::<T>::new(Some(base_columns)));
                let new_id = ArchetypeId::new(&comps);
                (new_id, new_storage)
            };
            let src = self.archetypes.get_mut(base_archetype).unwrap();
            src.move_to(*base_index, dest.as_mut());
            let new_index = try_cast_mut::<T>(dest.get_mut(comp_id).unwrap())
                .unwrap()
                .push(value);
            let new_ref = EntityRef {
                archetype: dest_id,
                index: new_index,
            };
            self.entities.insert(entity, new_ref);
            self.archetypes.insert(dest_id, dest);
        }
        Ok(())
    }

    pub fn get<T>(&self, entity: EntityId) -> Option<&T>
    where
        T: Default + 'static,
    {
        let e_ref = self.entities.get(&entity)?;
        let storage = self.archetypes.get(&e_ref.archetype)?;
        let column = storage.get(ComponentId::new::<T>())?;
        let typed = try_cast::<T>(column)?;
        typed.get(e_ref.index)
    }

    pub fn remove(&mut self, entity: EntityId) {
        unimplemented!()
    }

    pub fn query1<T>(&self) -> impl Iterator<Item = &T>
    where
        T: Default + 'static,
    {
        let comp_id = ComponentId::new::<T>();
        self.archetypes
            .iter()
            .map(move |(_, v)| v.get(comp_id))
            .filter(|v| v.is_some())
            .map(|v| try_cast::<T>(v.unwrap()).unwrap())
            .flat_map(|v| v.iter())
    }

    pub fn query2_mut<T1, T2>(&mut self) -> impl Iterator<Item = (&mut T1, &mut T2)>
    where
        T1: Default + 'static,
        T2: Default + 'static,
    {
        let comp_id1 = ComponentId::new::<T1>();
        let comp_id2 = ComponentId::new::<T2>();
        self.archetypes
            .iter_mut()
            .map(move |(_, v)| (v.get_mut(comp_id1), v.get_mut(comp_id2)))
            .filter(|(v1, v2)| v1.is_some() && v2.is_some())
            .map(|(v1, v2)| (cast_mut::<T1>(v1.unwrap()), cast_mut::<T2>(v1.unwrap())))
            .flat_map(|(v1, v2)| izip!(v1.iter_mut(), v2.iter_mut()))
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
        entities.set::<String>(e1, "test".to_owned()).unwrap();

        entities.set::<i32>(e2, 456).unwrap();
        entities.set::<f64>(e2, 5.5).unwrap();
        entities
            .set::<String>(e2, "yep yep yep".to_owned())
            .unwrap();

        assert_eq!(123, *entities.get::<i32>(e1).unwrap());
        assert_eq!(3.14, *entities.get::<f64>(e1).unwrap());
        assert_eq!("test".to_owned(), *entities.get::<String>(e1).unwrap());

        assert_eq!(456, *entities.get::<i32>(e2).unwrap());
        assert_eq!(5.5, *entities.get::<f64>(e2).unwrap());
        assert_eq!(
            "yep yep yep".to_owned(),
            *entities.get::<String>(e2).unwrap()
        );

        assert_eq!(
            vec!["test", "yep yep yep"],
            entities.query1::<String>().collect::<Vec<_>>()
        );
    }
}
