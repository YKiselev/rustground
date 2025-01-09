use std::{
    any::Any,
    cell::RefMut,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

trait Accessor {
    type ItemRef<'b>
    where
        Self: 'b;

    fn len(self) -> usize;

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
        = &'b mut T
    where
        Self: 'b;

    fn len(self) -> usize {
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
        = &'b T
    where
        Self: 'b;

    fn len(self) -> usize {
        self.slice.len()
    }

    fn get_at(&mut self, index: usize) -> Self::ItemRef<'_> {
        &self.slice[index]
    }
}

// struct AccessBuilder {
//     head: Option<Accessor>,
//     tail: Option<AccessBuilder>,
// }

// impl AccessBuilder {
//     fn new() -> Self {
//         Self {
//             head: None,
//             tail: None,
//         }
//     }

//     fn with_mut<T>() -> Self {
//         Self{
//             head: MutRef::new(slice)
//         }
//     }
// }

/*
struct Visitor1<A> {
    //handler: H,
    _phantom: PhantomData<A>,
}

impl<A> Visitor1<A>
where
    //H: Fn(A),
    for <'a> A: Accessor<ItemRef = A>
{
    fn new() -> Self {
        Visitor1 {
            //handler,
            _phantom: PhantomData::default(),
        }
    }

    fn visit<'a>(&self, s1: <A as Accessor<_>>::Slice) {
        let len = A::len(s1);
        for i in 0..len {
            let v1 = A::get_at(s1, i);
            //(self.handler)(v1);
        }
    }
}*/

#[cfg(test)]
mod test {

    use std::{
        any::Any,
        default,
        marker::PhantomData,
        ops::{Deref, DerefMut},
        sync::{Arc, Mutex},
    };

    use crate::playground::SharedRef;

    use super::{Accessor, MutRef};

    #[test]
    fn accessor() {
        let mut data1: Vec<i32> = vec![1, 2, 3];
        let slice1 = &mut data1[..];
        let mut data2: Vec<f32> = vec![1.0, 2.0, 3.0];
        let slice2 = &data2[..];

        let mut all = (MutRef::new(slice1), SharedRef::new(slice2));

        let v = all.0.get_at(1);
        assert_eq!(*v, 2);
        *v = 123;
        assert_eq!(*v, 123);
        let v = all.0.get_at(0);
        assert_eq!(*v, 1);
        *v = 222;
        assert_eq!(*v, 222);

        assert_eq!(*all.1.get_at(0), 1.0);
        assert_eq!(*all.1.get_at(2), 3.0);
    }

    #[test]
    fn visitor() {
        let clos1 = |a: &mut i32| {
            *a = 321;
        };
        //let _ = Visitor1::new(clos1);
        //let _ = Visitor1::<&mut i32>::new();//|_: &mut i32| {});
        //let _ = Visitor1::new(sys2);
    }
}
