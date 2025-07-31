use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use crate::{
    archetype::Chunk,
    component::{cast, cast_mut, ComponentId, ComponentStorage},
};

trait Arg {
    type Guard<'r>;
    type Item<'r>;
    type Iter<'i>: Iterator<Item = Self::Item<'i>>;

    fn lock<'a>(chunk: &'a Chunk) -> Self::Guard<'a>;

    fn iter<'a>(guard: &'a mut Self::Guard<'_>) -> Self::Iter<'a>;
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
}

#[cfg(test)]
mod tests {
    use crate::{
        archetype::{ArchetypeStorage, Chunk}, build_archetype, component::{cast, ComponentId}, entity::EntityId, v2::Arg
    };

    fn visit1<'a, F, A>(chunk: &'a Chunk, mut system: F)
    where
        for <'b> F: FnMut(A) + FnMut(<A as Arg>::Item<'b>),
        A:Arg,
        //for<'b>A:  Arg<Item<'b> = A> +'b,
    {
        let mut guard = A::lock(chunk);
        let mut it1 = A::iter(&mut guard);
        while let Some(v) = it1.next() {
            (system)(v);
        }
    }

    #[derive(Default, Debug)]
    struct Position(i32,i32);

    #[derive(Default, Debug)]
    struct Direction(i8,i8);

    #[test]
    fn test() {
        println!("Testing...");
        let mut storage = ArchetypeStorage::new(build_archetype![Direction, Position, f64, bool, i32], 1000);
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
}
