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
    type Iter<'i>: Iterator;

    fn lock(chunk: &Chunk) -> Self::Guard<'_>;

    fn iter<'a>(guard: &'a mut Self::Guard<'_>) -> Self::Iter<'a>;
}

struct MutRef<T> {
    _phantom: PhantomData<T>,
}

impl<T> Locker for MutRef<T>
where
    T: 'static,
{
    type Ty = T;
    type Guard<'g> = RwLockWriteGuard<'g, Box<dyn ComponentStorage>>;

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
    H: Fn(A::Iter<'_>),
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
        (self.handler)(it1);
    }
}

#[cfg(test)]
mod test {

    use std::{
        alloc::Layout,
        any::Any,
        marker::PhantomData,
        ops::{Deref, DerefMut},
        sync::{Arc, Mutex},
    };

    use crate::{archetype::ArchetypeStorage, build_archetype, entity::EntityId};

    use super::{CompRef, MutRef, Visitor1};

    #[test]
    fn visitor() {
        let vis = Visitor1::<MutRef<i32>, _>::new(|mut it1| {
            while let Some(v1) = it1.next() {
                dbg!(v1);
            }
        });

        let mut storage = ArchetypeStorage::new(build_archetype![String, f64, bool, i32], 1000);
        for i in 0..1000 {
            storage.add(EntityId::new(i));
        }
        for chunk in storage.iter() {
            vis.visit(chunk);
        }
    }
}
