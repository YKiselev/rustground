use std::{any::TypeId, collections::HashMap, hash::Hash};

use crate::component::{ComponentId, ComponentStorage, TypedComponentStorage};

#[derive(Default)]
struct ArchetypeBuilder {
    data: HashMap<ComponentId, Box<dyn ComponentStorage>>,
}

impl ArchetypeBuilder {
    fn add<T: Default + 'static>(mut self) -> Self {
        let component_id = ComponentId::new::<T>();
        self.data.insert(
            component_id,
            Box::new(TypedComponentStorage::<T>::default()),
        );
        self
    }

    fn build(mut self) -> ArchetypeStorage {
        ArchetypeStorage { data: self.data }
    }
}

#[derive(PartialEq, Eq, Hash)]
pub struct ArchetypeId(u64);

impl ArchetypeId {
    fn new(components: &Vec<Box<dyn ComponentStorage>>) -> Self {
        //components.iter().map(|v| TypeId::)
        ArchetypeId(0)
    }
}

pub struct ArchetypeStorage {
    data: HashMap<ComponentId, Box<dyn ComponentStorage>>,
}

impl ArchetypeStorage {}

impl Hash for ArchetypeStorage {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for k in self.data.keys() {
            k.hash(state);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::component::{ComponentStorage, TypedComponentStorage};

    use super::{ArchetypeBuilder, ArchetypeId};

    #[test]
    fn test() {
        let storage = ArchetypeBuilder::default()
            .add::<i32>()
            .add::<String>()
            .build();
    }
}
