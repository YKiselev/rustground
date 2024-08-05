use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use crate::VariableError::NotFound;
use crate::vars::VarRegistryError::{BadName, VarError};

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

struct Var {
    arc: Arc<Mutex<dyn VarBag + Send + Sync>>,
    info: Vec<VarInfo>,
}

#[derive(Default)]
pub struct VarRegistry {
    by_name: HashMap<String, Var>,
    by_type: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl VarRegistry {
    pub fn register<T: VarBag + Send + Sync + 'static>(&mut self, name: &str, value: &Arc<Mutex<T>>) -> Result<(), VarRegistryError> {
        let id = TypeId::of::<T>();
        if self.by_type.contains_key(&id) {
            return Err(VarRegistryError::TypeClash);
        }
        let owned_name = name.to_string();
        if self.by_name.contains_key(&owned_name) {
            return Err(VarRegistryError::NameClash);
        }
        let var = Var {
            arc: value.clone(),
            info: value.lock().unwrap().get_vars(),
        };
        self.by_type.insert(id, value.clone());
        self.by_name.insert(owned_name, var);
        Ok(())
    }

    pub fn get<T: VarBag + Send + 'static>(&self) -> Option<&Mutex<T>> {
        match self.by_type.get(&TypeId::of::<T>()) {
            None => None,
            Some(arc) => {
                arc.downcast_ref::<Mutex<T>>()
            }
        }
    }

    pub fn try_get_value(&self, name: &str) -> Option<String> {
        let mut sp = name.split("::");
        let bag = sp.next()?;
        let var_name = sp.next()?;
        if sp.next().is_none() {
            let var = self.by_name.get(bag)?;
            var.arc.lock().unwrap().try_get_var(var_name)
        } else {
            None
        }
    }

    pub fn try_set_value(&self, name: &str, value: &str) -> Result<(), VarRegistryError> {
        let mut sp = name.split("::");
        let bag = sp.next().ok_or(BadName)?;
        let var_name = sp.next().ok_or(BadName)?;
        if sp.next().is_none() {
            let var = self.by_name.get(bag).ok_or(VarError(NotFound))?;
            var.arc.lock().unwrap().try_set_var(var_name, value).map_err(|e| VarError(e))
        } else {
            Err(BadName)
        }
    }

    pub fn complete(&self, part: &str) -> Vec<String> {
        let mut sp = part.split("::");
        let bag_part = sp.next().or(Some("*")).unwrap();
        let var_part = sp.next();
        let bags = self.by_name.iter().filter(|(key, val)|
            key.starts_with(bag_part)
        );
        if var_part.is_none() {
            return bags.map(|(key, var)| key.clone()).collect();
        }
        let var_part = var_part.unwrap();
        bags.flat_map(|(key, var)|
            var.info.iter().filter(|v|
                v.name.starts_with(var_part)
            ).map(|v| key.to_string() + "::" + v.name)
        ).collect()
    }
}

#[derive(Debug, PartialEq, Eq)]
enum VarRegistryError {
    TypeClash,
    NameClash,
    BadName,
    VarError(VariableError),
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
            BadName => {
                write!(f, "Bad name!")
            }
            VarError(e) => {
                write!(f, "Variable error: {:?}!", e)
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
            NotFound => {
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
        pub(crate) speed: f64,
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
            speed: 345.466,
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
            speed: 234.567,
        }));
        let r = reg.register("a", &a);
        assert_eq!(r, Ok(()));

        let b = Arc::new(Mutex::new(TestVars::default()));
        let r = reg.register("b", &b);
        assert_eq!(r, Err(VarRegistryError::TypeClash));

        let c = Arc::new(Mutex::new(MoreTestVars::default()));
        let r = reg.register("a", &c);
        assert_eq!(r, Err(VarRegistryError::NameClash));

        {
            let a2 = reg.get::<TestVars>().unwrap().lock().unwrap();
            assert_eq!("my name", a2.name);
            assert_eq!(123, a2.counter);
        }
        {
            let mut ag = a.lock().unwrap();
            ag.counter = 1_000;
            ag.name = "altered name".to_string();
        }
        {
            let a2 = reg.get::<TestVars>().unwrap().lock().unwrap();
            assert_eq!("altered name", a2.name);
            assert_eq!(1_000, a2.counter);
        }
        assert_eq!("altered name", reg.try_get_value("a::name").unwrap());
        assert_eq!("1000", reg.try_get_value("a::counter").unwrap());
        assert_eq!("234.567", reg.try_get_value("a::speed").unwrap());

        reg.try_set_value("a::speed", "3.14").unwrap();
        assert_eq!("3.14", reg.try_get_value("a::speed").unwrap());

        let v = reg.complete("a");
        assert_eq!(v, ["a"]);

        let v = reg.complete("a::n");
        assert_eq!(v, ["a::name"]);
    }
}