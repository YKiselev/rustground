use std::{collections::hash_map::Values, marker::PhantomData, slice::Iter, sync::{RwLock, RwLockReadGuard}};

use crate::{archetype::{ArchetypeId, ArchetypeStorage, Chunk}, entity::{Entities, EntityStorage}};

///
/// Query
///
pub(crate) trait Query {
    type Item;

    fn next(&mut self) -> Option<Self::Item>;
}
///
///
///
pub(crate) struct MyQuery<'a, T>
where
    T: 'a,
{
    guard: RwLockReadGuard<'a, EntityStorage>,
    archetypes: Option<Values<'a, ArchetypeId, RwLock<ArchetypeStorage>>>,
    chunks: Option<Iter<'a, Chunk>>,
    _phantom: PhantomData<T>,
}


///
///
///
impl<'a, A> Query for MyQuery<'a, (A,)> {
    type Item = (A,);

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use std::marker::PhantomData;

    use crate::entity::Entities;

    use super::{MyQuery, Query};

    fn query<'a, T>(entities: &'a Entities) -> MyQuery<'a, T> {
        let guard = entities.read();
        //let archetypes = guard.archetypes();
        MyQuery {
            guard,
            archetypes: None,
            chunks: None,
            _phantom: PhantomData::default(),
        }
    }

    #[test]
    fn test() {
        let entities = Entities::new(8 * 1024);
        let q1 = query::<&i32>(&entities);
    }
}
