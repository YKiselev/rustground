use std::cmp;

use crate::{
    archetype::Chunk,
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

#[cfg(test)]
mod test {

    use super::{visit_2, visit_2b, visit_3};

    #[test]
    fn visitor() {
        let _ = visit_2::<i32, f64, _>(|_| {});
        let _ = visit_2b::<i32, f64, _>(|_| {});
        let _ = visit_3::<i32, f64, i64, _>(|_, _, _| {});
    }
}
