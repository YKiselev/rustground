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
pub struct Tuple1<T>
where
    T: Default + 'static,
{
    comp_id: ComponentId,
    handler: Box<dyn Fn(&T) -> ()>,
    _phantom: PhantomData<T>,
}

impl<T: Default + 'static> Tuple1<T> {
    pub fn new<H>(handler: H) -> Self
    where
        H: Fn(&T) -> () + 'static,
    {
        Tuple1 {
            comp_id: ComponentId::new::<T>(),
            handler: Box::new(handler),
            _phantom: PhantomData::default(),
        }
    }
}

impl<T: Default + 'static> Visitor for Tuple1<T> {
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
pub struct Tuple2<T1, T2>
where
    T1: Default + 'static,
    T2: Default + 'static,
{
    comp_id1: ComponentId,
    comp_id2: ComponentId,
    handler: Box<dyn Fn((&T1, &T2)) -> ()>,
    _phantom: PhantomData<(T1, T2)>,
}

impl<T1: Default + 'static, T2: Default + 'static> Tuple2<T1, T2> {
    pub fn new<H>(handler: H) -> Self
    where
        H: Fn((&T1, &T2)) -> () + 'static,
    {
        Tuple2 {
            comp_id1: ComponentId::new::<T1>(),
            comp_id2: ComponentId::new::<T2>(),
            handler: Box::new(handler),
            _phantom: PhantomData::default(),
        }
    }
}

impl<T1: Default + 'static, T2: Default + 'static> Visitor for Tuple2<T1, T2> {
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
