use std::{
    any::Any,
    borrow::{Borrow, BorrowMut},
    collections::HashSet,
    marker::PhantomData,
    ops::Index,
    slice::Iter,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use crate::{
    archetype::Chunk,
    component::{cast, cast_mut, ComponentId, ComponentStorage, TypedComponentStorage},
};

///
/// Component reference
///
trait CompRef {
    fn component_id() -> ComponentId;
}

impl<T: 'static> CompRef for &T {
    fn component_id() -> ComponentId {
        ComponentId::new::<T>()
    }
}

impl<T: 'static> CompRef for &mut T {
    fn component_id() -> ComponentId {
        ComponentId::new::<T>()
    }
}

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


/*
struct MutRef<T> {
    _phantom: PhantomData<T>,
}

impl<T> Locker for MutRef<T>
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

impl<T: 'static> CompRef for MutRef<T> {
    fn component_id() -> ComponentId {
        ComponentId::new::<T>()
    }
}

struct Ref<T> {
    _phantom: PhantomData<T>,
}

impl<T> Locker for Ref<T>
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

impl<T: 'static> CompRef for Ref<T> {
    fn component_id() -> ComponentId {
        ComponentId::new::<T>()
    }
}
*/

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
            component: ComponentId::new::<A::Ty>(),
            handler,
            _phantom: PhantomData::default(),
        }
    }

    fn accept(&self, columns: &HashSet<ComponentId>) -> bool {
        columns.contains(&self.component)
        //self.components.iter().all(|c| columns.contains(c))
    }

    fn visit(&self, chunk: &Chunk) {
        let mut guard1 = A::lock(chunk);
        let mut it1 = A::iter(&mut guard1);
        while let Some(v1) = it1.next() {
            (self.handler)(v1);
        }
    }
}

#[cfg(test)]
mod test {

    use crate::{archetype::ArchetypeStorage, build_archetype, entity::EntityId};

    use super::{Visitor1};

    #[test]
    fn visitor() {
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
}
