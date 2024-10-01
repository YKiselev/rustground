use std::{
    any::{Any, TypeId},
    fmt::Debug,
    hash::Hash,
    slice::{Iter, IterMut},
};

///
/// ComponentId
///
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct ComponentId(TypeId);

impl ComponentId {
    pub fn new<T: Default + 'static>() -> Self {
        ComponentId(TypeId::of::<T>())
    }
}

///
/// CoponentStorage trait
///
pub trait ComponentStorage {
    fn id(&self) -> ComponentId;

    fn row_count(&self) -> usize;

    fn as_any(&self) -> &dyn Any;

    fn as_mut_any(&mut self) -> &mut dyn Any;

    fn create_new(&self) -> Box<dyn ComponentStorage>;

    fn add(&mut self) -> usize;

    fn remove(&mut self, index: usize);

    fn move_to(&mut self, index: usize, dest: &mut dyn ComponentStorage);
}

///
/// Helper functions
///
#[inline]
pub(crate) fn try_cast<'a, T: Default + 'static>(
    value: &'a dyn ComponentStorage,
) -> Option<&'a TypedComponentStorage<T>> {
    value.as_any().downcast_ref::<TypedComponentStorage<T>>()
}

#[inline]
pub(crate) fn try_cast_mut<'a, T: Default + 'static>(
    value: &'a mut dyn ComponentStorage,
) -> Option<&'a mut TypedComponentStorage<T>> {
    value
        .as_mut_any()
        .downcast_mut::<TypedComponentStorage<T>>()
}

#[inline(always)]
pub(crate) fn cast<'a, T: Default + 'static>(
    value: &'a dyn ComponentStorage,
) -> &'a TypedComponentStorage<T> {
    try_cast::<T>(value).unwrap()
}

#[inline(always)]
pub(crate) fn cast_mut<'a, T: Default + 'static>(
    value: &'a mut dyn ComponentStorage,
) -> &'a mut TypedComponentStorage<T> {
    try_cast_mut(value).unwrap()
}

///
/// TypedComponentStorage
///
pub(crate) struct TypedComponentStorage<T: Default> {
    id: ComponentId,
    data: Vec<T>,
}

impl<T: Default + 'static> TypedComponentStorage<T> {
    pub(crate) fn new(capacity: usize) -> Self {
        TypedComponentStorage {
            id: ComponentId::new::<T>(),
            data: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub(crate) fn push(&mut self, value: T) -> usize {
        self.data.push(value);
        self.data.len() - 1
    }

    #[inline(always)]
    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        self.data.get(index)
    }

    #[inline(always)]
    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.data.get_mut(index)
    }

    #[inline(always)]
    pub(crate) fn set(&mut self, index: usize, value: T) {
        self.data[index] = value;
    }

    #[inline(always)]
    pub(crate) fn iter(&self) -> Iter<'_, T> {
        self.data.iter()
    }

    #[inline(always)]
    pub(crate) fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.data.iter_mut()
    }

    #[inline(always)]
    pub(crate) fn slice(&self) -> &[T] {
        &self.data
    }

}

impl<T: Any + Default + 'static> ComponentStorage for TypedComponentStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn create_new(&self) -> Box<dyn ComponentStorage> {
        Box::new(TypedComponentStorage::<T>::new(self.data.capacity()))
    }

    fn id(&self) -> ComponentId {
        self.id
    }

    fn add(&mut self) -> usize {
        self.data.push(T::default());
        self.data.len() - 1
    }

    fn move_to(&mut self, index: usize, dest: &mut dyn ComponentStorage) {
        let opt = if index + 1 < self.data.len() {
            Some(self.data.swap_remove(index))
        } else {
            self.data.pop()
        };
        if let Some(value) = opt {
            cast_mut::<T>(dest).push(value);
        }
    }

    fn remove(&mut self, index: usize) {
        if index + 1 < self.data.len() {
            self.data.swap_remove(index);
        } else if index < self.data.len() {
            self.data.pop();
        }
    }

    fn row_count(&self) -> usize {
        self.data.len()
    }
}

///
/// Tests
///
#[cfg(test)]
mod test {
    use super::{ComponentId, ComponentStorage, TypedComponentStorage};

    #[derive(Copy, Clone, Default, Debug, PartialEq)]
    struct A {
        pub x: f32,
        pub y: f32,
    }

    #[test]
    fn component_id() {
        let i1 = ComponentId::new::<i32>();
        let i2 = ComponentId::new::<i32>();
        assert_eq!(i1, i2);
        let i3 = ComponentId::new::<i16>();
        assert_ne!(i1, i3);
    }

    #[test]
    fn test() {
        let mut columns: Vec<Box<dyn ComponentStorage>> = vec![
            Box::new(TypedComponentStorage::<i32>::new(128)),
            Box::new(TypedComponentStorage::<A>::new(128)),
        ];

        let s1 = columns[0].as_mut();

        let t1 = s1
            .as_mut_any()
            .downcast_mut::<TypedComponentStorage<i32>>()
            .unwrap();
        t1.push(1);
        t1.push(2);
        t1.push(3);

        assert_eq!(1, *t1.get(0).unwrap());
        assert_eq!(2, *t1.get(1).unwrap());
        assert_eq!(3, *t1.get(2).unwrap());
        assert_eq!(None, t1.get(3));

        let s2 = columns[1].as_mut();
        let t2 = s2
            .as_mut_any()
            .downcast_mut::<TypedComponentStorage<A>>()
            .unwrap();
        t2.push(A { x: 1., y: 2. });
        assert_eq!(A { x: 1., y: 2. }, *t2.get(0).unwrap());
    }
}
