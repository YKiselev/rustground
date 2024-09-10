use std::{
    any::{Any, TypeId},
    collections::HashSet,
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

    fn add_row(&mut self) -> usize;

    fn move_to(&mut self, index: usize, dest: &mut dyn ComponentStorage);

    fn get(&self, comp_id: ComponentId) -> Option<&dyn ComponentStorage>;

    fn get_mut(&mut self, comp_id: ComponentId) -> Option<&mut dyn ComponentStorage>;

    fn collect_components(&self, result: &mut HashSet<ComponentId>);

    fn components(&self) -> HashSet<ComponentId> {
        let mut result = HashSet::new();
        self.collect_components(&mut result);
        result
    }
}

///
/// Helper functions
///
pub(crate) fn try_cast<'a, T: Default + 'static>(
    value: &'a dyn ComponentStorage,
) -> Option<&'a TypedComponentStorage<T>> {
    value.as_any().downcast_ref::<TypedComponentStorage<T>>()
}

pub(crate) fn try_cast_mut<'a, T: Default + 'static>(
    value: &'a mut dyn ComponentStorage,
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
    next: Option<Box<dyn ComponentStorage>>,
}

impl<T: Default + 'static> TypedComponentStorage<T> {
    pub(crate) fn new(next: Option<Box<dyn ComponentStorage>>) -> Self {
        TypedComponentStorage {
            id: ComponentId::new::<T>(),
            data: Vec::new(),
            next,
        }
    }

    pub(crate) fn push(&mut self, value: T) -> usize {
        self.data.push(value);
        self.data.len() - 1
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

impl<T: Any + Default + 'static> ComponentStorage for TypedComponentStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn create_new(&self) -> Box<dyn ComponentStorage> {
        Box::new(TypedComponentStorage::<T>::new(
            self.next.as_ref().map(|n| n.create_new()),
        ))
    }

    fn id(&self) -> ComponentId {
        self.id
    }

    fn add_row(&mut self) -> usize {
        self.data.push(T::default());
        if let Some(n) = self.next.as_mut() {
            n.add_row();
        }
        self.data.len() - 1
    }

    fn move_to(&mut self, index: usize, dest: &mut dyn ComponentStorage) {
        if let Some(storage) = dest.get_mut(self.id) {
            if let Some(value) = if index + 1 < self.data.len() {
                Some(self.data.swap_remove(index))
            } else {
                self.data.pop()
            } {
                try_cast_mut::<T>(storage).unwrap().push(value);
            }
        }
        if let Some(n) = self.next.as_mut() {
            n.move_to(index, dest)
        }
    }

    fn get(&self, comp_id: ComponentId) -> Option<&dyn ComponentStorage> {
        if self.id == comp_id {
            Some(self)
        } else {
            self.next.as_ref().and_then(|n| n.get(comp_id))
        }
    }

    fn get_mut(&mut self, comp_id: ComponentId) -> Option<&mut dyn ComponentStorage> {
        if self.id == comp_id {
            Some(self)
        } else {
            self.next.as_mut().and_then(|n| n.get_mut(comp_id))
        }
    }

    fn collect_components(&self, result: &mut HashSet<ComponentId>) {
        result.insert(self.id);
        if let Some(n) = self.next.as_ref() {
            n.collect_components(result);
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
            Box::new(TypedComponentStorage::<i32>::new(None)),
            Box::new(TypedComponentStorage::<A>::new(None)),
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
