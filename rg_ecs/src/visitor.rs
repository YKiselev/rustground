use std::{fmt::Debug, marker::PhantomData};

use itertools::izip;

use crate::{
    archetype::{Archetype, Chunk},
    component::{cast_mut, ComponentId},
};

///
/// Entities visitor
///
pub trait Visitor {
    fn accept(&self, archetype: &Archetype) -> bool;
    fn visit(&self, chunk: &Chunk);
}

///
/// Tuple1
///
pub struct Tuple1<'a, T>
where
    T: Default + 'static,
    //H: Fn(&T) + 'a
{
    comp_id: ComponentId,
    handler: Box<dyn Fn(&T) + 'a>,
}

impl<'a, T: Default + 'static> Tuple1<'a, T> {
    pub fn new<H>(handler: H) -> Self
    where
        H: Fn(&T) + 'a,
    {
        Tuple1 {
            comp_id: ComponentId::new::<T>(),
            handler: Box::new(handler),
        }
    }
}

impl<'a, T: Default + 'static> Visitor for Tuple1<'a, T> {
    fn accept(&self, archetype: &Archetype) -> bool {
        archetype.has_component(&self.comp_id)
    }

    fn visit(&self, chunk: &Chunk) {
        let lock = chunk.get_column(self.comp_id).unwrap();
        let mut guard = lock.write().unwrap();
        let iter = cast_mut::<T>(guard.as_mut()).iter();
        for v in iter {
            (self.handler)(v);
        }
    }
}

///
/// Tuple2
///
pub struct Tuple2<'a, T1, T2>
where
    T1: Default + 'static,
    T2: Default + 'static,
{
    comp_id1: ComponentId,
    comp_id2: ComponentId,
    handler: Box<dyn Fn((&T1, &T2)) + 'a>,
}

impl<'a, T1: Default + 'static, T2: Default + 'static> Tuple2<'a, T1, T2> {
    pub fn new<H>(handler: H) -> Self
    where
        H: Fn((&T1, &T2)) -> () + 'a,
    {
        Tuple2 {
            comp_id1: ComponentId::new::<T1>(),
            comp_id2: ComponentId::new::<T2>(),
            handler: Box::new(handler),
        }
    }
}

impl<'a, T1: Default + 'static, T2: Default + 'static> Visitor for Tuple2<'a, T1, T2> {
    fn accept(&self, archetype: &Archetype) -> bool {
        archetype.has_component(&self.comp_id1) && archetype.has_component(&self.comp_id2)
    }

    fn visit(&self, chunk: &Chunk) {
        let lock1 = chunk.get_column(self.comp_id1).unwrap();
        let lock2 = chunk.get_column(self.comp_id2).unwrap();
        let mut guard1 = lock1.write().unwrap();
        let mut guard2 = lock2.write().unwrap();
        let iter1 = cast_mut::<T1>(guard1.as_mut()).iter();
        let iter2 = cast_mut::<T2>(guard2.as_mut()).iter();
        let iter = izip!(iter1, iter2);
        for v in iter {
            (self.handler)(v);
        }
    }
}

pub fn visit_2<T1, T2, H>(handler: H) -> impl Fn(&Chunk)
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
        let iter1 = cast_mut::<T1>(guard1.as_mut()).iter();
        let iter2 = cast_mut::<T2>(guard2.as_mut()).iter();
        let iter = izip!(iter1, iter2);
        for v in iter {
            (handler)(v);
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::{AtomicI64, Ordering};

    use super::{visit_2, Tuple1, Visitor};

    #[test]
    fn visitor() {
        let counter = AtomicI64::default();
        let _ = Tuple1::<String>::new(|_| {
            counter.fetch_add(1, Ordering::Relaxed);
        });

        let v2 = visit_2::<i32, f64,_>(|ch| {});
    }
}
