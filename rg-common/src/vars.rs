use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};

#[derive(Debug, Default)]
pub struct VarInfo {
    pub name: &'static str,
    pub persisted: bool,
}

pub trait VarBag {
    fn get_vars(&self) -> Vec<VarInfo>;

    fn try_get_var(&self, name: &str) -> Option<String>;

    fn try_set_var(&mut self, name: &str, value: &str) -> Result<(), VariableError>;
}

#[derive(Default)]
pub struct VarRegistry {
    name2type: HashMap<String, TypeId>,
    data: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl VarRegistry {
    pub fn register<T: VarBag + Send + 'static>(&mut self, name: &str, value: Arc<Mutex<T>>) -> Result<(), VarRegistryError> {
        let id = TypeId::of::<T>();
        if self.data.contains_key(&id) {
            return Err(VarRegistryError::TypeClash);
        }
        let owned_name = name.to_string();
        if self.name2type.contains_key(&owned_name) {
            return Err(VarRegistryError::NameClash);
        }
        self.data.insert(id, value);
        self.name2type.insert(owned_name, id);
        Ok(())
    }

    pub fn get<T: VarBag + Send + 'static>(&self) -> Option<Arc<Mutex<T>>> {
        match self.data.get(&TypeId::of::<T>()) {
            None => None,
            Some(arc) => {
                Some(arc.clone().downcast::<Mutex<T>>().unwrap())
            }
        }
    }

    pub fn get2<T: VarBag + Send + 'static>(&self) -> Option<&Mutex<T>> {
        match self.data.get(&TypeId::of::<T>()) {
            None => None,
            Some(arc) => {
                Some(arc.downcast_ref::<Mutex<T>>().unwrap())
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum VarRegistryError {
    TypeClash,
    NameClash,
}

impl Display for VarRegistryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VarRegistryError::TypeClash => {
                write!(f, "Type id already registered!")
            }
            VarRegistryError::NameClash => {
                write!(f, "Name already used!")
            }
        }
    }
}

impl Error for VarRegistryError {}

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd, Clone)]
pub enum VariableError {
    ParsingError,
    NotFound,
}

impl Display for VariableError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VariableError::ParsingError => {
                write!(f, "Parsing failed!")
            }
            VariableError::NotFound => {
                write!(f, "No such variable!")
            }
        }
    }
}

impl Error for VariableError {}


#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::fmt::Display;
    use std::sync::{Arc, Mutex};

    use rg_common::vars::VarRegistryError;
    use rg_macros::VarBag;

    use crate::vars::{VarBag, VarRegistry};

    #[derive(VarBag, Default)]
    pub(crate) struct TestVars {
        pub(crate) counter: i32,
        #[transient]
        pub(crate) flag: bool,
        pub(crate) name: String,
    }

    #[derive(VarBag, Default)]
    pub(crate) struct MoreTestVars {
        pub(crate) speed: f32,
    }

    #[test]
    fn var_bag() {
        let mut v = TestVars {
            flag: false,
            counter: 123,
            name: "some name".to_string(),
        };
        let infos = v.get_vars()
            .into_iter()
            .map(|v| (v.name, v))
            .collect::<HashMap<_, _>>();

        let info = infos.get("flag").unwrap();
        assert_eq!(false, info.persisted);
        let info = infos.get("counter").unwrap();
        assert_eq!(true, info.persisted);
        let info = infos.get("name").unwrap();
        assert_eq!(true, info.persisted);

        assert_eq!("false", v.try_get_var("flag").unwrap());
        assert_eq!("123", v.try_get_var("counter").unwrap());
        assert_eq!("some name", v.try_get_var("name").unwrap());
        assert_eq!(None, v.try_get_var("unknown"));

        v.try_set_var("flag", "true").unwrap();
        v.try_set_var("name", "New name").unwrap();
        v.try_set_var("counter", "321").unwrap();

        assert_eq!("true", v.try_get_var("flag").unwrap());
        assert_eq!("321", v.try_get_var("counter").unwrap());
        assert_eq!("New name", v.try_get_var("name").unwrap());
    }

    #[test]
    fn var_registry() {
        let mut reg = VarRegistry::default();
        let a = Arc::new(Mutex::new(TestVars {
            counter: 123,
            flag: false,
            name: "my name".to_string(),
        }));
        let r = reg.register("a", a.clone());
        assert_eq!(r, Ok(()));

        let b = Arc::new(Mutex::new(TestVars::default()));
        let r = reg.register("b", b);
        assert_eq!(r, Err(VarRegistryError::TypeClash));

        let c = Arc::new(Mutex::new(MoreTestVars::default()));
        let r = reg.register("a", c);
        assert_eq!(r, Err(VarRegistryError::NameClash));

        {
            let binding = reg.get::<TestVars>().unwrap();
            let a2 = binding.lock().unwrap();
            assert_eq!("my name", a2.name);
            assert_eq!(123, a2.counter);
        }
        {
            let mut ag = a.lock().unwrap();
            ag.counter = 1_000;
            ag.name = "altered name".to_string();
        }
        {
            let a2 = reg.get2::<TestVars>().unwrap().lock().unwrap();
            assert_eq!("altered name", a2.name);
            assert_eq!(1_000, a2.counter);
        }
    }
}