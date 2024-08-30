use std::{
    any::{Any, TypeId},
    hash::{DefaultHasher, Hash, Hasher},
    ptr::hash,
};

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct ComponentId(u64);

impl ComponentId {
    pub fn new<T: 'static>() -> Self {
        let type_id = TypeId::of::<T>();
        let mut hasher = DefaultHasher::new();
        type_id.hash(&mut hasher);
        ComponentId(hasher.finish())
    }
}

pub(crate) struct ComponentStorageFactory {
    pub id: ComponentId,
    create: Box<dyn Fn() -> Box<dyn ComponentStorage>>,
}

impl ComponentStorageFactory {
    pub fn new<T: Default + 'static>() -> Self {
        ComponentStorageFactory {
            id: ComponentId::new::<T>(),
            create: Box::new(|| Box::new(TypedComponentStorage::<T>::default())),
        }
    }

    pub fn create(&self) -> Box<dyn ComponentStorage> {
        (self.create)()
    }
}

pub trait ComponentStorage {
    fn as_any(&self) -> &dyn Any;

    fn as_mut_any(&mut self) -> &mut dyn Any;
}

#[derive(Default)]
pub(crate) struct TypedComponentStorage<T: Default> {
    data: Vec<T>,
}

impl<T: Default> TypedComponentStorage<T> {
    fn push(&mut self, value: T) {
        self.data.push(value);
    }

    fn get(&self, index: usize) -> Option<&T> {
        self.data.get(index)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.data.get_mut(index)
    }
}

impl<T: Any + Default + Default + 'static> ComponentStorage for TypedComponentStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod test {
    use super::{ComponentStorage, TypedComponentStorage};

    #[derive(Copy, Clone, Default, Debug, PartialEq)]
    struct A {
        pub x: f32,
        pub y: f32,
    }

    struct Make {
        factory: Box<dyn Fn() -> Box<dyn ComponentStorage>>,
    }

    impl Make {
        fn new<T: Default + 'static>() -> Self {
            Make {
                factory: Box::new(|| Box::new(TypedComponentStorage::<T>::default())),
            }
        }
    }

    #[test]
    fn test() {
        let mut columns: Vec<Box<dyn ComponentStorage>> = vec![
            Box::new(TypedComponentStorage::<i32>::default()),
            Box::new(TypedComponentStorage::<A>::default()),
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

        let m = Make::new::<f32>();
        let mut s = (m.factory)();
        let t3 = s
            .as_mut_any()
            .downcast_mut::<TypedComponentStorage<f32>>()
            .unwrap();
        t3.push(3.2);
        assert_eq!(3.2, *t3.get(0).unwrap())
    }
}
