use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::IsTerminal;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use toml::Table;

use crate::VariableError::NotFound;
use crate::vars::VarRegistryError::{BadName, VarError};

#[derive(Debug, Default)]
pub struct VarInfo {
    pub name: &'static str,
    pub persisted: bool,
}

pub enum Variable<'a> {
    VarBag(&'a dyn VarBag),
    String(&'a str),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

pub trait VarBag {
    fn get_vars(&self) -> Vec<VarInfo>;

    fn try_get_var(&self, name: &str) -> Option<Variable<'_>>;

    fn try_set_var(&mut self, name: &str, value: &str) -> Result<(), VariableError>;
}

pub trait FromStrMutator {
    fn set_from_str(&mut self, value: &str) -> Result<(), VariableError>;
}

struct Var {
    arc: Arc<Mutex<dyn VarBag + Send + Sync>>,
    info: Vec<VarInfo>,
}

#[derive(Default)]
pub struct VarRegistry {
    persisted: Option<Table>,
    by_name: HashMap<String, Var>,
    by_type: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl VarRegistry {
    fn load_values<T: VarBag>(&self, name: &str, mut bag: &Arc<Mutex<T>>, var: &Var) {
        if let Some(table) = self.persisted.as_ref().map(|t| t.get(name)).flatten() {
            var.info.iter().filter(|v|
                v.persisted
            ).for_each(|v|
                if let Some(value) = table.get(v.name) {
                    let mut guard = bag.lock().unwrap();
                    if let Err(e) = guard.try_set_var(v.name, &value.to_string()) {
                        // ???
                    }
                }
            );
        }
    }

    fn save_all_values(&self) {}

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
        self.load_values(name, value, &var);
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
            var.arc.lock().unwrap().try_get_var(var_name).map(|v|
                match v {
                    Variable::VarBag(b) => {
                        "<bag>".to_string()
                    }
                    Variable::String(v) => {
                        v.to_string()
                    }
                    Variable::Integer(v) => {
                        v.to_string()
                    }
                    Variable::Float(v) => {
                        v.to_string()
                    }
                    Variable::Boolean(v) => {
                        v.to_string()
                    }
                }
            )
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
    use std::fmt::{Debug, Display};
    use std::str::FromStr;
    use std::sync::{Arc, Mutex};

    use rg_common::vars::VarRegistryError;
    use rg_macros::VarBag;

    use crate::VariableError;
    use crate::vars::{FromStrMutator, VarBag, Variable, VarRegistry};

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

        assert_eq!("false", v.try_get_var("flag").unwrap().to_string());
        assert_eq!("123", v.try_get_var("counter").unwrap().to_string());
        assert_eq!("some name", v.try_get_var("name").unwrap().to_string());
        assert!(v.try_get_var("unknown").is_none());

        v.try_set_var("flag", "true").unwrap();
        v.try_set_var("name", "New name").unwrap();
        v.try_set_var("counter", "321").unwrap();

        assert_eq!("true", v.try_get_var("flag").unwrap().to_string());
        assert_eq!("321", v.try_get_var("counter").unwrap().to_string());
        assert_eq!("New name", v.try_get_var("name").unwrap().to_string());
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

    #[derive(Debug, VarBag)]
    struct Sub {
        name: String,
        counter: i32,
    }

    #[derive(VarBag)]
    struct Outer {
        sub: Sub,
        speed: f32,
        flag: bool,
    }

    #[test]
    fn config() {
        let mut c = Outer {
            sub: Sub {
                name: "test".to_string(),
                counter: 1,
            },
            speed: 3.22,
            flag: true,
        };
        let v = Variable::from(&c);
        assert!(matches!(v, Variable::VarBag{..}));
        let v = Variable::from(&c.sub.counter);
        assert!(matches!(v, Variable::Integer{..}));
        let v = Variable::from(&c.sub);
        assert!(matches!(v, Variable::VarBag{..}));
        let v = Variable::from(&c.sub.name);
        assert!(matches!(v, Variable::String{..}));
        let v = Variable::from(&c.speed);
        assert!(matches!(v, Variable::Float{..}));
        let v = Variable::from(&c.flag);
        assert!(matches!(v, Variable::Boolean{..}));

        c.sub.counter.set_from_str("321").unwrap();
        assert_eq!(c.sub.counter, 321);
        c.speed.set_from_str("3.33").unwrap();
        assert_eq!(c.speed, 3.33);
    }
}