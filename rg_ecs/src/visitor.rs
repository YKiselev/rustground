use std::{
    collections::HashSet,
    marker::PhantomData,
    slice::Iter,
    sync::{RwLockReadGuard, RwLockWriteGuard},
};

use crate::{
    archetype::Chunk,
    component::{cast, cast_mut, ComponentId, ComponentStorage},
};

///
/// Locker
///
trait Locker {
    type Ty: 'static;
    type Guard<'g>;
    type Item<'r>;
    type Iter<'i>: Iterator<Item = Self::Item<'i>>;

    fn lock(chunk: &Chunk) -> Self::Guard<'_>;

    fn iter<'a>(guard: &'a mut Self::Guard<'_>) -> Self::Iter<'a>;
}

impl<T> Locker for &mut T
where
    T: 'static,
{
    type Ty = T;
    type Guard<'g> = RwLockWriteGuard<'g, Box<dyn ComponentStorage>>;
    type Item<'r> = &'r mut T;
    type Iter<'i> = core::slice::IterMut<'i, T>;

    fn lock(chunk: &Chunk) -> Self::Guard<'_> {
        chunk
            .get_column(ComponentId::new::<T>())
            .unwrap()
            .write()
            .unwrap()
    }

    fn iter<'a>(guard: &'a mut Self::Guard<'_>) -> Self::Iter<'a> {
        cast_mut::<T>(guard.as_mut()).iter_mut()
    }
}

impl<T> Locker for &T
where
    T: 'static,
{
    type Ty = T;
    type Guard<'g> = RwLockReadGuard<'g, Box<dyn ComponentStorage>>;
    type Item<'r> = &'r T;
    type Iter<'i> = core::slice::Iter<'i, T>;

    fn lock(chunk: &Chunk) -> Self::Guard<'_> {
        chunk
            .get_column(ComponentId::new::<T>())
            .unwrap()
            .read()
            .unwrap()
    }

    fn iter<'a>(guard: &'a mut Self::Guard<'_>) -> Self::Iter<'a> {
        cast::<T>(guard.as_ref()).iter()
    }
}

fn comp_id<L: Locker>() -> ComponentId {
    ComponentId::new::<L::Ty>()
}

///
/// Visitor1
///
struct Visitor1<A, H> {
    component: ComponentId,
    handler: H,
    _phantom: PhantomData<A>,
}

impl<A, H> Visitor1<A, H>
where
    H: Fn(A::Item<'_>),
    A: Locker,
{
    fn new(handler: H) -> Self {
        Visitor1 {
            component: comp_id::<A>(),
            handler,
            _phantom: PhantomData::default(),
        }
    }

    fn accept(&self, columns: &HashSet<ComponentId>) -> bool {
        columns.contains(&self.component)
    }

    fn visit(&self, chunk: &Chunk) {
        let mut guard1 = A::lock(chunk);
        let mut it1 = A::iter(&mut guard1);
        while let Some(v1) = it1.next() {
            (self.handler)(v1);
        }
    }
}

///
/// Visitor2
///
struct Visitor2<A, B, H> {
    components: Vec<ComponentId>,
    handler: H,
    _phantom: PhantomData<(A, B)>,
}

impl<A, B, H> Visitor2<A, B, H>
where
    H: Fn(A::Item<'_>, B::Item<'_>),
    A: Locker,
    B: Locker,
{
    fn new(handler: H) -> Self {
        Visitor2 {
            components: vec![comp_id::<A>(), comp_id::<B>()],
            handler,
            _phantom: PhantomData::default(),
        }
    }

    fn accept(&self, columns: &HashSet<ComponentId>) -> bool {
        self.components.iter().all(|c| columns.contains(c))
    }

    fn visit(&self, chunk: &Chunk) {
        let mut guard1 = A::lock(chunk);
        let mut guard2 = B::lock(chunk);
        let mut it1 = A::iter(&mut guard1);
        let mut it2 = B::iter(&mut guard2);
        while let (Some(v1), Some(v2)) = (it1.next(), it2.next()) {
            (self.handler)(v1, v2);
        }
    }
}


#[cfg(test)]
mod test {

    use crate::{archetype::ArchetypeStorage, build_archetype, entity::EntityId};

    use super::{Visitor1, Visitor2};

    #[test]
    fn visitor1() {
        let mut storage = ArchetypeStorage::new(build_archetype![String, f64, bool, i32], 1000);
        for i in 0..5 {
            storage.add(EntityId::new(i));
        }

        let vis = Visitor1::<&mut i32, _>::new(|v1| {
            dbg!(v1);
        });

        for chunk in storage.iter() {
            vis.visit(chunk);
        }

        let vis = Visitor1::<&f64, _>::new(|v1| {
            dbg!(v1);
        });

        for chunk in storage.iter() {
            vis.visit(chunk);
        }
    }

    #[test]
    fn visitor2() {
        let mut storage = ArchetypeStorage::new(build_archetype![String, f64, bool, i32], 1000);
        for i in 0..5 {
            storage.add(EntityId::new(i));
        }

        let vis = Visitor2::<&mut i32, &f64, _>::new(|v1, v2| {
            dbg!(v1, v2);
        });

        for chunk in storage.iter() {
            vis.visit(chunk);
        }
    }
}
