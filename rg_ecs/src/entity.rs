use std::collections::HashMap;

use crate::archetype::{ArchetypeId, ArchetypeStorage};


pub struct EntityId(usize);

pub struct Entities {
    archetypes: HashMap<ArchetypeId, Box<ArchetypeStorage>>
}

#[cfg(test)]
mod test {

    #[test]
    fn test() {

    }
}