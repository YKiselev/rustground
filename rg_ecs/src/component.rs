use std::{
    any::{Any, TypeId},
    hash::Hash,
};

///
/// ComponentId
///
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct ComponentId(TypeId);

impl ComponentId {
    pub fn new<T: 'static>() -> Self {
        ComponentId(TypeId::of::<T>())
    }
}

///
/// CoponentStorage trait
///
pub trait ComponentStorage {
    fn id(&self) -> ComponentId;

    fn as_any(&self) -> &dyn Any;

    fn as_mut_any(&mut self) -> &mut dyn Any;

    fn create_new(&self) -> Box<dyn ComponentStorage>;

    fn push(&mut self) -> usize;

    fn move_to(&mut self, index: usize, dest: &mut dyn ComponentStorage) -> bool;
}

///
/// Helper functions
///
pub(crate) fn try_cast<'a, T: Default + 'static>(
    value: &'a Box<dyn ComponentStorage>,
) -> Option<&'a TypedComponentStorage<T>> {
    value.as_any().downcast_ref::<TypedComponentStorage<T>>()
}

pub(crate) fn try_cast_mut<'a, T: Default + 'static>(
    value: &'a mut Box<dyn ComponentStorage>,
) -> Option<&'a mut TypedComponentStorage<T>> {
    value
        .as_mut_any()
        .downcast_mut::<TypedComponentStorage<T>>()
}

///
/// TypedComponentStorage
///
pub(crate) struct TypedComponentStorage<T: Default> {
    id: ComponentId,
    data: Vec<T>,
}

impl<T: Default + 'static> TypedComponentStorage<T> {
    pub(crate) fn new() -> Self {
        TypedComponentStorage {
            id: ComponentId::new::<T>(),
            data: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, value: T) {
        self.data.push(value);
    }

    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        self.data.get(index)
    }

    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.data.get_mut(index)
    }

    pub(crate) fn set(&mut self, index: usize, value: T) {
        self.data[index] = value;
    }
}

impl<T: Any + Default + Default + 'static> ComponentStorage for TypedComponentStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn create_new(&self) -> Box<dyn ComponentStorage> {
        Box::new(TypedComponentStorage::<T>::new())
    }

    fn id(&self) -> ComponentId {
        self.id
    }

    fn push(&mut self) -> usize {
        self.data.push(T::default());
        self.data.len() - 1
    }

    fn move_to(&mut self, index: usize, dest: &mut dyn ComponentStorage) -> bool {
        if let Some(storage) = dest.as_mut_any().downcast_mut::<TypedComponentStorage<T>>() {
            storage.push(self.data.swap_remove(index));
            true
        } else {
            false
        }
    }
}

///
/// Tests
///
#[cfg(test)]
mod test {
    use super::{ComponentStorage, TypedComponentStorage};

    #[derive(Copy, Clone, Default, Debug, PartialEq)]
    struct A {
        pub x: f32,
        pub y: f32,
    }

    #[test]
    fn test() {
        let mut columns: Vec<Box<dyn ComponentStorage>> = vec![
            Box::new(TypedComponentStorage::<i32>::new()),
            Box::new(TypedComponentStorage::<A>::new()),
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
