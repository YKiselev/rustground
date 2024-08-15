use std::borrow::Cow;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::iter::Peekable;
use std::ops::Deref;
use std::str::Split;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::VariableError::NotFound;
use crate::vars::VarRegistryError::VarError;

pub enum Variable<'a> {
    VarBag(&'a dyn VarBag),
    String(Cow<'a, str>),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    None
}

pub trait VarBag {
    fn get_vars(&self) -> Vec<String>;

    fn try_get_var(&self, name: &str) -> Option<Variable<'_>>;

    fn try_set_var(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError>;
}

pub trait FromStrMutator {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError>;
}

#[derive(Default)]
pub struct VarRegistry<T> where T: VarBag {
    data: Option<Arc<Mutex<T>>>,
}

impl<T: VarBag> VarRegistry<T> {
    
    pub const DELIMITER: &'static str  = "::";

    pub fn new(data: Arc<Mutex<T>>) -> Self {
        VarRegistry { data: Some(data) }
    }

    pub fn set_data(&mut self, config: Arc<Mutex<T>>) {
        self.data = Some(config);
    }

    pub fn get_data(&self) -> Option<&Arc<Mutex<T>>> {
        self.data.as_ref()
    }
    pub fn get_mut_data(&mut self) -> Option<&mut Arc<Mutex<T>>> {
        self.data.as_mut()
    }

    fn lock_data(&self) -> Option<MutexGuard<T>> {
        self.data.as_ref()?.lock().ok()
    }

    pub fn try_get_value(&self, name: &str) -> Option<String> {
        let guard = self.lock_data()?;
        let mut v = Variable::from(guard.deref());
        let mut sp = name.split("::");
        loop {
            match v {
                Variable::VarBag(bag) => {
                    v = bag.try_get_var(sp.next()?)?;
                }
                Variable::String(s) => {
                    return if sp.next().is_none() {
                        Some(s.to_string())
                    } else {
                        None
                    };
                }
                Variable::Integer(i) => {
                    return if sp.next().is_none() {
                        Some(i.to_string())
                    } else {
                        None
                    };
                }
                Variable::Float(f) => {
                    return if sp.next().is_none() {
                        Some(f.to_string())
                    } else {
                        None
                    };
                }
                Variable::Boolean(b) => {
                    return if sp.next().is_none() {
                        Some(b.to_string())
                    } else {
                        None
                    };
                }
                Variable::None => {
                    return if sp.next().is_none() {
                        Some("None".to_string())
                    } else {
                        None
                    };
                }
            }
        }
    }

    pub fn try_set_value(&self, name: &str, value: &str) -> Result<(), VarRegistryError> {
        let mut sp = name.split("::");
        let mut guard = self.lock_data().ok_or(VarRegistryError::LockFailed)?;
        guard.try_set_var(&mut sp, value)?;
        Ok(())
    }

    fn filter_names(owner: &dyn VarBag, sp: &mut Peekable<Split<&str>>, prefix: &str, result: &mut Vec<String>) {
        if let Some(part) = sp.next() {
            if part.is_empty() {
                return;
            }
            for var_name in owner.get_vars() {
                if !var_name.starts_with(part) {
                    continue;
                }
                if let Some(v) = owner.try_get_var(&var_name) {
                    let local_prefix = if !prefix.is_empty() {
                        prefix.to_string() + "::" + &var_name
                    } else {
                        var_name.clone()
                    };
                    if sp.peek().is_none() {
                        result.push(local_prefix.clone());
                    }
                    match v {
                        Variable::VarBag(value) => {
                            Self::filter_names(value, sp, local_prefix.as_str(), result)
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn complete(&self, part: &str) -> Option<Vec<String>> {
        let mut sp = part.split("::").peekable();
        self.lock_data().map(|guard| {
            let mut result = Vec::new();
            Self::filter_names(guard.deref(), &mut sp, "", &mut result);
            result
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum VarRegistryError {
    VarError(VariableError),
    LockFailed,
}

impl Display for VarRegistryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VarError(e) => {
                write!(f, "Variable error: {:?}!", e)
            }
            VarRegistryError::LockFailed => {
                write!(f, "Lock failed!")
            }
        }
    }
}

impl Error for VarRegistryError {}

impl From<VariableError> for VarRegistryError {
    fn from(value: VariableError) -> Self {
        VarRegistryError::VarError(value)
    }
}

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
    use std::collections::HashSet;
    use std::fmt::Debug;
    use std::str::Split;
    use std::sync::{Arc, Mutex};

    use rg_macros::VarBag;

    use crate::vars::{FromStrMutator, VarBag, Variable, VarRegistry};

    #[derive(VarBag, Default)]
    struct TestVars {
        counter: i32,
        flag: bool,
        name: String,
        speed: f64,
        sub: MoreTestVars,
    }

    #[derive(VarBag, Default)]
    struct MoreTestVars {
        speed: f32,
    }

    #[test]
    fn var_bag() {
        let mut v = TestVars {
            flag: false,
            counter: 123,
            name: "some name".to_string(),
            speed: 345.466,
            sub: MoreTestVars {
                speed: 330.0
            },
        };
        let infos = v.get_vars()
            .into_iter()
            //.map(|v| (v.name, true))
            .collect::<HashSet<_>>();


        assert_eq!("false", v.try_get_var("flag").unwrap().to_string());
        assert_eq!("123", v.try_get_var("counter").unwrap().to_string());
        assert_eq!("some name", v.try_get_var("name").unwrap().to_string());
        assert!(v.try_get_var("unknown").is_none());

        v.try_set_var(&mut "flag".split("::"), "true").unwrap();
        v.try_set_var(&mut "name".split("::"), "New name").unwrap();
        v.try_set_var(&mut "counter".split("::"), "321").unwrap();

        assert_eq!("true", v.try_get_var("flag").unwrap().to_string());
        assert_eq!("321", v.try_get_var("counter").unwrap().to_string());
        assert_eq!("New name", v.try_get_var("name").unwrap().to_string());
    }

    #[test]
    fn var_registry() {
        let mut reg = VarRegistry::default();
        let root = Arc::new(Mutex::new(TestVars {
            counter: 123,
            flag: false,
            name: "my name".to_string(),
            speed: 234.567,
            sub: MoreTestVars {
                speed: 220.0
            },
        }));
        reg.set_data(root);
        assert_eq!("my name", reg.try_get_value("name").unwrap());
        assert_eq!("123", reg.try_get_value("counter").unwrap());
        assert_eq!("234.567", reg.try_get_value("speed").unwrap());
        assert_eq!("false", reg.try_get_value("flag").unwrap());
        assert_eq!("220", reg.try_get_value("sub::speed").unwrap());

        reg.try_set_value("sub::speed", "5").unwrap();
        assert_eq!("5", reg.try_get_value("sub::speed").unwrap());

        let v = reg.complete("s").unwrap();
        assert_eq!(v, ["speed", "sub"]);

        let v = reg.complete("s::s").unwrap();
        assert_eq!(v, ["sub::speed"]);
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

    fn empty_split() -> Split<'static, &'static str> {
        let mut result = "".split("::");
        result.next();
        result
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

        c.sub.counter.set_from_str(&mut empty_split(), "321").unwrap();
        assert_eq!(c.sub.counter, 321);
        c.speed.set_from_str(&mut empty_split(), "3.33").unwrap();
        assert_eq!(c.speed, 3.33);
    }
}