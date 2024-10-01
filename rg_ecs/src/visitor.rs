use std::{cmp, fmt::Debug, marker::PhantomData};

use itertools::izip;

use crate::{
    archetype::{Archetype, Chunk},
    component::{cast_mut, ComponentId},
};

pub fn visit_1<T1, H>(handler: H) -> impl Fn(&Chunk) -> usize
where
    T1: Default + 'static,
    H: Fn(&T1),
{
    move |chunk| {
        let lock1 = chunk.get_column(ComponentId::new::<T1>()).unwrap();
        let mut guard1 = lock1.write().unwrap();
        let s1 = cast_mut::<T1>(guard1.as_mut()).slice();
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
        let s1 = cast_mut::<T1>(guard1.as_mut()).slice();
        let s2 = cast_mut::<T2>(guard2.as_mut()).slice();
        let len = cmp::min(s1.len(), s2.len());
        let mut row_count = 0;
        for i in 0..len {
            let v1 = &s1[i];
            let v2 = &s2[i];
            (handler)((v1, v2));
            row_count += 1;
        }
        row_count
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::{AtomicI64, Ordering};

    use super::visit_2;

    #[test]
    fn visitor() {
        let _ = visit_2::<i32, f64, _>(|ch| {});
    }
}
