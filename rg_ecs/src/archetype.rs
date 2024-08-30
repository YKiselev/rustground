use std::{
    any::TypeId,
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use crate::component::{
    ComponentId, ComponentStorage, ComponentStorageFactory, TypedComponentStorage,
};

#[derive(Default)]
struct ArchetypeBuilder {
    data: Vec<ComponentStorageFactory>,
}

impl ArchetypeBuilder {
    fn add<T: Default + 'static>(mut self) -> Self {
        self.data.push(ComponentStorageFactory::new::<T>());
        self
    }

    fn build(mut self) -> Archetype {
        let mut hasher = DefaultHasher::new();
        for f in self.data.iter() {
            f.id.hash(&mut hasher);
        }
        Archetype {
            id: ArchetypeId(hasher.finish()),
            factories: self.data,
        }
    }
}

pub(crate) struct Archetype {
    id: ArchetypeId,
    factories: Vec<ComponentStorageFactory>,
}

impl Archetype {
    fn create_storage(&self) -> ArchetypeStorage {
        ArchetypeStorage {
            id: self.id,
            data: self.factories.iter().map(|f| (f.id, f.create())).collect(),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct ArchetypeId(u64);

pub(crate) struct ArchetypeStorage {
    pub id: ArchetypeId,
    data: Vec<(ComponentId, Box<dyn ComponentStorage>)>,
}

impl ArchetypeStorage {
    
}


#[cfg(test)]
mod test {
    use crate::component::{ComponentStorage, TypedComponentStorage};

    use super::{ArchetypeBuilder, ArchetypeId};

    #[test]
    fn test() {
        let archetype = ArchetypeBuilder::default()
            .add::<i32>()
            .add::<String>()
            .build();
        println!("Got id: {:?}", archetype.id);
        let storage = archetype.create_storage();
    }
}
