use std::{
    collections::HashSet,
    marker::PhantomData,
    sync::{RwLockReadGuard, RwLockWriteGuard},
};

use crate::{
    archetype::Chunk,
    component::{cast, cast_mut, ComponentId, ComponentStorage},
};

pub trait Arg {
    type Guard<'r>;
    type Item<'r>;
    type Iter<'i>: Iterator<Item = Self::Item<'i>>;

    fn lock<'a>(chunk: &'a Chunk) -> Self::Guard<'a>;

    fn iter<'a>(guard: &'a mut Self::Guard<'_>) -> Self::Iter<'a>;

    fn comp_id() -> ComponentId;
}

impl<T> Arg for &T
where
    T: 'static,
{
    type Guard<'g> = RwLockReadGuard<'g, Box<dyn ComponentStorage>>;
    type Item<'r> = &'r T;
    type Iter<'i> = core::slice::Iter<'i, T>;

    fn lock<'a>(chunk: &'a Chunk) -> Self::Guard<'a> {
        chunk
            .get_column(ComponentId::new::<T>())
            .unwrap()
            .read()
            .unwrap()
    }

    fn iter<'a>(guard: &'a mut Self::Guard<'_>) -> Self::Iter<'a> {
        cast::<T>(guard.as_ref()).iter()
    }

    fn comp_id() -> ComponentId {
        ComponentId::new::<T>()
    }
}

impl<T> Arg for &mut T
where
    T: 'static,
{
    type Guard<'g> = RwLockWriteGuard<'g, Box<dyn ComponentStorage>>;
    type Item<'r> = &'r mut T;
    type Iter<'i> = core::slice::IterMut<'i, T>;

    fn lock<'a>(chunk: &'a Chunk) -> Self::Guard<'a> {
        chunk
            .get_column(ComponentId::new::<T>())
            .unwrap()
            .write()
            .unwrap()
    }

    fn iter<'a>(guard: &'a mut Self::Guard<'_>) -> Self::Iter<'a> {
        cast_mut::<T>(guard.as_mut()).iter_mut()
    }

    fn comp_id() -> ComponentId {
        ComponentId::new::<T>()
    }
}

pub trait Visitor {
    //fn accept(&self, columns: &HashSet<ComponentId>) -> bool;

    fn visit(&mut self, chunk: &Chunk);
}

pub trait AsVisitor<Args> {
    fn as_visitor(self) -> impl Visitor;
}

struct SystemFn<F, Args>(F, Vec<ComponentId>, PhantomData<Args>)
where
    F: FnMut(&Chunk);

impl<F, Args> Visitor for SystemFn<F, Args>
where
    F: FnMut(&Chunk),
{
    fn visit(&mut self, chunk: &Chunk) {
        (self.0)(chunk)
    }
}

// impl<F, A> AsVisitor<(A,)> for F
// where
//     for<'b> F: FnMut(A) + FnMut(<A as Arg>::Item<'b>),
//     A: Arg,
// {
//     fn as_visitor(mut self) -> impl Visitor {
//         let f = move |chunk: &Chunk| {
//             let mut guard = A::lock(chunk);
//             let mut it1 = A::iter(&mut guard);
//             while let Some(v) = it1.next() {
//                 (self)(v);
//             }
//         };
//         SystemFn::<_, (A,)>(f, vec![A::comp_id()], PhantomData::default())
//     }
// }

macro_rules! impl_as_visitor {
    ($($t:ident),+) => {
        impl<F, $($t),*> AsVisitor<($($t,)*)> for F
        where
            for<'b> F: FnMut($($t),*) + FnMut($(<$t as Arg>::Item<'b>),*),
            $(
                $t: Arg
            ),*
        {
            paste::paste! {
            fn as_visitor(mut self) -> impl Visitor {
                let f = move |chunk: &Chunk| {
                    $(
                    let mut [<guard_ $t:lower>] = $t::lock(chunk);
                    )*
                    $(
                    let mut [<it_ $t:lower>] = $t::iter(&mut [<guard_ $t:lower>]);
                    )*
                    while let ($(Some([<v_ $t:lower>]),)*) = ($([<it_ $t:lower>].next(),)*) {
                        (self)($([<v_ $t:lower>]),*);
                    }
                };
                SystemFn::<_, ($($t,)*)>(f, vec![$($t::comp_id()),*], PhantomData::default())
            }}
        }
    };
}

impl_as_visitor!(A);
impl_as_visitor!(A, B);
impl_as_visitor!(A, B, C);
impl_as_visitor!(A, B, C, D);
impl_as_visitor!(A, B, C, D, E);

#[cfg(test)]
mod tests {
    use crate::{
        archetype::{ArchetypeStorage, Chunk},
        build_archetype,
        component::{cast, ComponentId},
        entity::EntityId,
        v2::{Arg, AsVisitor, Visitor},
    };

    fn visit1<'a, F, A>(chunk: &'a Chunk, mut system: F)
    where
        for<'b> F: FnMut(A) + FnMut(<A as Arg>::Item<'b>),
        A: Arg,
        //for<'b>A:  Arg<Item<'b> = A> +'b,
    {
        let mut guard = A::lock(chunk);
        let mut it1 = A::iter(&mut guard);
        while let Some(v) = it1.next() {
            (system)(v);
        }
    }

    #[derive(Default, Debug)]
    struct Position(i32, i32);

    #[derive(Default, Debug)]
    struct Direction(i8, i8);

    #[test]
    fn test() {
        let mut storage =
            ArchetypeStorage::new(build_archetype![Direction, Position, f64, bool, i32], 1000);
        for i in 0..10 {
            storage.add(EntityId::new(i));
        }

        let show_system = |a: &Position| {
            println!("Got {a:?}");
        };
        let modify_system = |a: &mut Position| {
            a.0 += 17;
            a.1 += 77;
        };

        // Pass 1
        for chunk in storage.iter() {
            visit1(chunk, show_system);
            visit1(chunk, modify_system);
        }

        // Pass 2
        for chunk in storage.iter() {
            visit1(chunk, show_system);
        }
    }

    #[test]
    fn as_visitor() {
        let show_system = |a: &Position| {
            println!("Got {a:?}");
        };
        let modify_system = |a: &mut Position| {
            a.0 += 17;
            a.1 += 77;
        };
        let mut v1 = show_system.as_visitor();
        let mut v2 = modify_system.as_visitor();

        let mut storage = ArchetypeStorage::new(
            build_archetype![Direction, Position, f64, bool, i32], 1000
        );
        for i in 0..10 {
            storage.add(EntityId::new(i));
        }
        // Pass 1
        for chunk in storage.iter() {
            v1.visit(chunk);
            v2.visit(chunk);
        }

        // Pass 2
        for chunk in storage.iter() {
            v1.visit(chunk);
        }
    }
}
