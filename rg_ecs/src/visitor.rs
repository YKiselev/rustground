use std::{
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
///
///
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

/**
 * Locker
 */
trait Locker {
    type Guard<'a>;
    type Slice<'a>;

    fn lock(chunk: &Chunk) -> Self::Guard<'_>;

    fn get_slice<'a>(guard: &'a mut Self::Guard<'_>) -> Self::Slice<'a>;
}

impl<T> Locker for &mut T
where
    T: 'static,
{
    type Guard<'a> = RwLockWriteGuard<'a, Box<dyn ComponentStorage>>;
    type Slice<'a> = MutRef<'a, T>;

    fn lock(chunk: &Chunk) -> Self::Guard<'_> {
        chunk
            .get_column(ComponentId::new::<T>())
            .unwrap()
            .write()
            .unwrap()
    }

    fn get_slice<'a>(guard: &'a mut Self::Guard<'_>) -> Self::Slice<'a> {
        MutRef::new(cast_mut::<T>(guard.as_mut()))
    }
}

impl<T> Locker for &T
where
    T: 'static,
{
    type Guard<'a> = RwLockReadGuard<'a, Box<dyn ComponentStorage>>;
    type Slice<'a> = SharedRef<'a, T>;

    fn lock(chunk: &Chunk) -> Self::Guard<'_> {
        chunk
            .get_column(ComponentId::new::<T>())
            .unwrap()
            .read()
            .unwrap()
    }

    fn get_slice<'a>(guard: &'a mut Self::Guard<'_>) -> Self::Slice<'a> {
        SharedRef::new(cast::<T>(guard.as_ref()))
    }
}

/**
 * Accessor trait
 */
trait Accessor {
    type ItemRef<'b>;
    // where
    //     Self: 'b;

    fn len(&self) -> usize;

    fn get_at(&mut self, index: usize) -> Self::ItemRef<'_>;
}

/**
 * Mutable reference
 */
struct MutRef<'a, T>
where
    T: 'static,
{
    slice: &'a mut [T],
}

impl<'a, T> MutRef<'a, T>
where
    T: 'static,
{
    fn new(slice: &'a mut [T]) -> Self {
        MutRef { slice }
    }
}

impl<'a, T> Accessor for MutRef<'a, T>
where
    T: 'static,
{
    type ItemRef<'b>
        = &'b mut T;
    // where
    //     Self: 'b;

    fn len(&self) -> usize {
        self.slice.len()
    }

    fn get_at(&mut self, index: usize) -> Self::ItemRef<'_> {
        &mut self.slice[index]
    }
}

/**
 * Shared reference
 */
struct SharedRef<'a, T>
where
    T: 'static,
{
    slice: &'a [T],
}

impl<'a, T> SharedRef<'a, T>
where
    T: 'static,
{
    fn new(slice: &'a [T]) -> Self {
        SharedRef { slice }
    }
}

impl<'a, T> Accessor for SharedRef<'a, T>
where
    T: 'static,
{
    type ItemRef<'b>
        = &'b T;
    // where
    //     Self: 'b;

    fn len(&self) -> usize {
        self.slice.len()
    }

    fn get_at(&mut self, index: usize) -> Self::ItemRef<'_> {
        &self.slice[index]
    }
}
/*
impl<T> Accessor for &T
where
    T: 'static,
{
    //type Guard<'a> = RwLockReadGuard<'a, Box<dyn ComponentStorage>>;
    //type Slice<'a> = &'a [T];
    type ItemRef<'a> = &'a T;

    // fn lock(chunk: &Chunk) -> Self::Guard<'_> {
    //     chunk
    //         .get_column(ComponentId::new::<T>())
    //         .unwrap()
    //         .read()
    //         .unwrap()
    // }

    // fn get_slice<'a>(guard: &'a mut Self::Guard<'_>) -> Self::Slice<'a> {
    //     cast::<T>(guard.as_ref())
    // }

    fn len(slice: &Self::Slice<'_>) -> usize {
        slice.len()
    }

    fn get_at<'a>(&mut self, index: usize) -> Self::ItemRef<'a> {
        &self.slice[index]
    }
}

impl<T> Accessor for &mut T
where
    T: 'static,
{
    //type Guard<'a> = RwLockWriteGuard<'a, Box<dyn ComponentStorage>>;
    //type Slice<'a> = &'a mut [T];
    type ItemRef<'a>
        = &'a mut T
    where
        Self: 'a;

    // fn lock(chunk: &Chunk) -> Self::Guard<'_> {
    //     chunk
    //         .get_column(ComponentId::new::<T>())
    //         .unwrap()
    //         .write()
    //         .unwrap()
    // }

    // fn get_slice<'a>(guard: &'a mut Self::Guard<'_>) -> Self::Slice<'a> {
    //     cast_mut::<T>(guard.as_mut())
    // }

    fn len(&self) -> usize {
        slice.len()
    }

    fn get_at<'a>(slice: &'a mut Self::Slice<'_>, index: usize) -> Self::ItemRef<'a> {
        &mut slice[index]
    }
}
*/
struct Visitor1<A, H> {
    component: ComponentId,
    handler: H,
    _phantom: PhantomData<A>,
}

impl<A, H> Visitor1<A, H>
where
    H: Fn(A),
    for<'a> A: CompRef + Locker,
    for<'a, 'b> <A as Locker>::Slice<'b>: Accessor<ItemRef<'a> = A>,
{
    fn new(handler: H) -> Self {
        Visitor1 {
            component: A::component_id(),
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
        let mut s1 = A::get_slice(&mut guard1);
        let len = s1.len();
        for i in 0..len {
            let v1 = s1.get_at(i);
            (self.handler)(v1);
        }
    }
}

#[cfg(test)]
mod test {

    use std::{
        any::Any,
        marker::PhantomData,
        ops::{Deref, DerefMut},
        sync::{Arc, Mutex},
    };

    use super::{Accessor, CompRef, Visitor1};

    #[test]
    fn visitor() {
        let _ = Visitor1::new(|_: &i32| {});
        // let _ = Visitor1::new(sys3);
    }
}
