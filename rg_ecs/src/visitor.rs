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

pub fn visit_1<T1, H>(handler: H) -> impl Fn(&Chunk) -> usize
where
    T1: Default + 'static,
    H: Fn(&T1),
{
    move |chunk| {
        let lock1 = chunk.get_column(ComponentId::new::<T1>()).unwrap();
        let mut guard1 = lock1.write().unwrap();
        let s1 = &cast_mut::<T1>(guard1.as_mut());
        let len = s1.len();
        let mut row_count = 0;
        for i in 0..len {
            let v1 = &s1[i];
            (handler)(v1);
            row_count += 1;
        }
        row_count
    }
}

pub fn visit_2<T1, T2, H>(handler: H) -> impl Fn(&Chunk) -> usize
where
    T1: Default + 'static,
    T2: Default + 'static,
    H: Fn((&T1, &T2)),
{
    move |chunk| {
        let lock1 = chunk.get_column(ComponentId::new::<T1>()).unwrap();
        let lock2 = chunk.get_column(ComponentId::new::<T2>()).unwrap();
        let mut guard1 = lock1.write().unwrap();
        let mut guard2 = lock2.write().unwrap();
        let s1 = &cast_mut::<T1>(guard1.as_mut());
        let s2 = &cast_mut::<T2>(guard2.as_mut());
        assert_eq!(s1.len(), s2.len());
        //let len = cmp::min(s1.len(), s2.len());
        //let s1 = &s1[..len];
        //let s2 = &s2[..len];
        let mut row_count = 0;
        for i in 0..s1.len() {
            let v1 = &s1[i];
            let v2 = &s2[i];
            (handler)((v1, v2));
            row_count += 1;
        }
        row_count
    }
}

pub fn visit_2b<T1, T2, H>(handler: H) -> impl Fn(&Chunk) -> usize
where
    T1: Default + 'static,
    T2: Default + 'static,
    H: Fn((&T1, &T2)),
{
    move |chunk| {
        let lock1 = chunk.get_column(ComponentId::new::<T1>()).unwrap();
        let lock2 = chunk.get_column(ComponentId::new::<T2>()).unwrap();
        let mut guard1 = lock1.write().unwrap();
        let mut guard2 = lock2.write().unwrap();
        let s1 = cast_mut::<T1>(guard1.as_mut()).iter();
        let s2 = cast_mut::<T2>(guard2.as_mut()).iter();
        let mut row_count = 0;
        for (v1, v2) in s1.zip(s2) {
            (handler)((v1, v2));
            row_count += 1;
        }
        row_count
    }
}

pub fn visit_3<T1, T2, T3, H>(handler: H) -> impl Fn(&Chunk) -> usize
where
    T1: Default + 'static,
    T2: Default + 'static,
    T3: Default + 'static,
    H: Fn(&mut T1, &mut T2, &mut T3),
{
    move |chunk| {
        let lock1 = chunk.get_column(ComponentId::new::<T1>()).unwrap();
        let lock2 = chunk.get_column(ComponentId::new::<T2>()).unwrap();
        let lock3 = chunk.get_column(ComponentId::new::<T3>()).unwrap();
        let mut guard1 = lock1.write().unwrap();
        let mut guard2 = lock2.write().unwrap();
        let mut guard3 = lock3.write().unwrap();
        let s1 = &mut cast_mut::<T1>(guard1.as_mut());
        let s2 = &mut cast_mut::<T2>(guard2.as_mut());
        let s3 = &mut cast_mut::<T3>(guard3.as_mut());
        assert_eq!(s1.len(), s2.len());
        assert_eq!(s1.len(), s3.len());
        //let len = cmp::min(s1.len(), s2.len());
        //let s1 = &s1[..len];
        //let s2 = &s2[..len];
        let mut row_count = 0;
        for i in 0..s1.len() {
            let v1 = &mut s1[i];
            let v2 = &mut s2[i];
            let v3 = &mut s3[i];
            (handler)(v1, v2, v3);
            row_count += 1;
        }
        row_count
    }
}

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

trait Accessor {
    type Guard<'a>;
    type Slice<'a>;
    type ItemRef<'a>;

    fn lock(chunk: &Chunk) -> Self::Guard<'_>;

    fn get_slice<'a>(guard: &'a mut Self::Guard<'_>) -> Self::Slice<'a>;

    fn len(slice: &Self::Slice<'_>) -> usize;

    fn get_at<'a>(slice: &'a mut Self::Slice<'_>, index: usize) -> Self::ItemRef<'a>;
}
/*
impl<T> Accessor for &T
where
    T: 'static,
{
    type Guard<'a> = RwLockReadGuard<'a, Box<dyn ComponentStorage>>;
    type Slice<'a> = &'a [T];
    type SliceItemRef<'a> = &'a T;

    fn lock(chunk: &Chunk) -> Self::Guard<'_> {
        chunk
            .get_column(ComponentId::new::<T>())
            .unwrap()
            .read()
            .unwrap()
    }

    fn get_slice<'a>(guard: &'a mut Self::Guard<'_>) -> Self::Slice<'a> {
        cast::<T>(guard.as_ref())
    }

    fn len(slice: &Self::Slice<'_>) -> usize {
        slice.len()
    }

    fn get_at<'a>(slice: &'a mut Self::Slice<'_>, index: usize) -> Self::SliceItemRef<'a> {
        &slice[index]
    }
}
*/
impl<T> Accessor for &mut T
where
    T: 'static,
{
    type Guard<'a> = RwLockWriteGuard<'a, Box<dyn ComponentStorage>>;
    type Slice<'a> = &'a mut [T];
    type ItemRef<'a> = &'a mut T;

    fn lock(chunk: &Chunk) -> Self::Guard<'_> {
        chunk
            .get_column(ComponentId::new::<T>())
            .unwrap()
            .write()
            .unwrap()
    }

    fn get_slice<'a>(guard: &'a mut Self::Guard<'_>) -> Self::Slice<'a> {
        cast_mut::<T>(guard.as_mut())
    }

    fn len(slice: &Self::Slice<'_>) -> usize {
        slice.len()
    }

    fn get_at<'a>(slice: &'a mut Self::Slice<'_>, index: usize) -> Self::ItemRef<'a> {
        &mut slice[index]
    }
}

struct Visitor1<A, H> {
    component: ComponentId,
    handler: H,
    _phantom: PhantomData<A>,
}

impl<A, H> Visitor1<A, H>
where
     H: Fn(A),
     for <'a> A: CompRef + Accessor<ItemRef<'a> = A> + 'a,
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
         let len = A::len(&s1);
         for i in 0..len {
             let v1 = A::get_at(&mut s1, i);
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

    use super::{visit_2, visit_2b, visit_3, Accessor, CompRef, Visitor1};

    fn sys2(a: &mut i32) {
        *a = 123;
    }

    fn sys3(a: &i32) {}

    // #[inline(never)]
    // fn access<'a, R>(mut slice: R::Slice<'a>, index: usize) -> R
    // where
    //     R: Accessor<SliceItemRef<'a> = R> + 'a,
    // {
    //     R::get_at(&mut slice, index)
    // }

    // fn access2<'a, R, H>(mut slice: R::Slice<'a>, index: usize, handler: H)
    // where
    //     R: Accessor<SliceItemRef<'a> = R> + 'a,
    //     H: Fn(R),
    // {
    //     (handler)(R::get_at(&mut slice, index));
    // }

    // struct V<A, H> {
    //     handler: H,
    //     _phantom: PhantomData<A>,
    // }

    // impl<'a, A, H> V<A, H>
    // where
    //     H: Fn(A),
    //     A: Accessor<SliceItemRef<'a> = A> + 'a,
    // {
    //     fn new(handler: H) -> Self {
    //         Self {
    //             handler,
    //             _phantom: PhantomData::default(),
    //         }
    //     }

    //     fn call_at(&self, mut slice: A::Slice<'a>, index: usize) {
    //         (self.handler)(A::get_at(&mut slice, index));
    //     }
    // }

    #[test]
    fn accessor() {
        let mut v: Vec<i32> = vec![1, 2, 3];
        let s = &mut v[..];
        // *access::<&mut i32>(s, 0) = 111;
        // *access::<&mut i32>(s, 0) = 222;
        // assert_eq!(222, *access::<&i32>(s, 0));
        // assert_eq!(222, *access::<&i32>(s, 0));

        // access2::<&mut i32, _>(s, 0, |v: &mut i32| *v = 222);
        // access2::<&mut i32, _>(s, 0, |v: &mut i32| *v = 333);
        // access2::<&mut i32, _>(s, 0, sys2);
        // access2::<&mut i32, _>(s, 0, sys2);

        // let vis = V::new(sys2);
        // vis.call_at(s, 0);
    }

    #[test]
    fn visit_N() {
        let _ = visit_2::<i32, f64, _>(|_| {});
        let _ = visit_2b::<i32, f64, _>(|_| {});
        let _ = visit_3::<i32, f64, i64, _>(|_, _, _| {});
    }

    #[test]
    fn visitor() {
        let clos1 = |a: &mut i32| {
            *a = 321;
        };
        // let _: Visitor1<&mut i32, _> = Visitor1::new(clos1);
        // let _: Visitor1<&mut i32, _> = Visitor1::new(|_: &mut i32| {});
        // let _: Visitor1<&mut i32, _> = Visitor1::new(sys2);

        let clos1 = |a: &i32| {};
        // let _ = Visitor1::new(clos1);
        //let _ = Visitor1::new(|_: &i32| {});
        // let _ = Visitor1::new(sys3);
    }
}
