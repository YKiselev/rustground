use std::borrow::Cow;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::iter::Peekable;
use std::ops::Deref;
use std::str::Split;
use std::sync::{Arc, Mutex, MutexGuard, RwLock, Weak};

use snafu::Snafu;

use crate::vars::VarRegistryError::VarError;

pub enum Variable<'a> {
    VarBag(&'a dyn VarBag),
    String(Cow<'a, str>),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    None,
}

pub trait VarBag {
    fn get_vars(&self) -> Vec<String>;

    fn try_get_var(&self, name: &str) -> Option<Variable<'_>>;

    fn try_set_var(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError>;
}

pub trait FromStrMutator {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError>;
}

type VarBagBox = Box<dyn VarBag + Send + Sync>;
type VarBagRef = Weak<RwLock<VarBagBox>>;
type VarBagMap = HashMap<String, VarBagRef>;

#[derive(Debug)]
pub struct VarRegistry(Mutex<VarBagMap>);

impl VarRegistry {
    pub const DELIMITER: &'static str = "::";

    pub fn new() -> Self {
        Self(Mutex::new(Default::default()))
    }

    pub fn add(&self, name: String, part: &Arc<RwLock<VarBagBox>>) -> Result<(), VarRegistryError> {
        let mut guard = self.lock().ok_or(VarRegistryError::LockFailed)?;
        match guard.entry(name) {
            Entry::Occupied(mut entry) => {
                if entry.get().upgrade().is_some() {
                    Err(VarRegistryError::AlreadyExists)
                } else {
                    entry.insert(Arc::downgrade(part));
                    Ok(())
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(Arc::downgrade(part));
                Ok(())
            }
        }
    }

    fn lock(&self) -> Option<MutexGuard<VarBagMap>> {
        self.0.lock().ok()
    }

    pub fn try_get_value<S>(&self, name: S) -> Option<String>
    where
        S: AsRef<str>,
    {
        let mut sp = name.as_ref().split(Self::DELIMITER);
        let guard = self.lock()?;
        let arc = guard.get(sp.next()?)?.upgrade()?;
        let v_guard = arc.read().ok()?;
        let mut v = Variable::from(v_guard.deref());

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
        let mut sp = name.split(Self::DELIMITER);
        let guard = self.lock().ok_or(VarRegistryError::LockFailed)?;
        let key = sp.next().ok_or(not_found())?;
        let arc = guard
            .get(key)
            .ok_or(not_found())?
            .upgrade()
            .ok_or(not_found())?;
        arc.write()
            .map_err(|_| VarRegistryError::LockFailed)?
            .try_set_var(&mut sp, value)?;
        Ok(())
    }

    pub fn complete(&self, part: &str) -> Option<Vec<String>> {
        let mut sp = part.split(Self::DELIMITER).peekable();
        let bag_name = sp.next()?;
        let guard = self.lock()?;
        let mut result = Vec::new();
        for (key, value) in guard.iter() {
            if !bag_name.is_empty() && !key.starts_with(bag_name) {
                continue;
            }
            if let Some(arc) = value.upgrade() {
                if let Ok(lr) = arc.read() {
                    let start = result.len();
                    filter_names(lr.deref().deref(), &mut sp, "", &mut result);
                    for v in result[start..].iter_mut() {
                        *v = key.clone() + VarRegistry::DELIMITER + v;
                    }
                }
            }
        }
        Some(result)
    }
}

fn not_found() -> VarRegistryError {
    VarRegistryError::VarError(VariableError::NotFound)
}

fn filter_names(
    owner: &dyn VarBag,
    sp: &mut Peekable<Split<&str>>,
    prefix: &str,
    result: &mut Vec<String>,
) {
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
                    prefix.to_string() + VarRegistry::DELIMITER + &var_name
                } else {
                    var_name.clone()
                };
                if sp.peek().is_none() {
                    result.push(local_prefix.clone());
                }
                match v {
                    Variable::VarBag(value) => {
                        filter_names(value, sp, local_prefix.as_str(), result)
                    }
                    _ => {}
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum VarRegistryError {
    VarError(VariableError),
    AlreadyExists,
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
            VarRegistryError::AlreadyExists => write!(f, "Already exists!"),
        }
    }
}

impl Error for VarRegistryError {}

impl From<VariableError> for VarRegistryError {
    fn from(value: VariableError) -> Self {
        VarRegistryError::VarError(value)
    }
}

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd, Clone, Snafu)]
pub enum VariableError {
    #[snafu(display("Parsing failed"))]
    ParsingError,
    #[snafu(display("Not found"))]
    NotFound,
}

impl Debug for dyn VarBag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut ds = f.debug_struct("VarBag");
        for name in self.get_vars() {
            ds.field(
                &name,
                &self
                    .try_get_var(&name)
                    .map(|v| v.to_string())
                    .unwrap_or("".to_owned()),
            );
        }
        ds.finish()
    }
}

pub fn wrap_var_bag<T>(value: T) -> Arc<RwLock<VarBagBox>>
where
    T: VarBag + Send + Sync + 'static,
{
    let boxed: VarBagBox = Box::new(value);
    Arc::new(RwLock::new(boxed))
}

#[cfg(test)]
mod test {
    use std::fmt::Debug;
    use std::str::Split;
    use std::sync::{Arc, RwLock};

    use rg_macros::VarBag;

    use super::*;

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
            sub: MoreTestVars { speed: 330.0 },
        };

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
        let reg = VarRegistry::new();
        let arc = wrap_var_bag(TestVars {
            counter: 123,
            flag: false,
            name: "my name".to_string(),
            speed: 234.567,
            sub: MoreTestVars { speed: 220.0 },
        });
        reg.add("root".to_owned(), &arc).unwrap();
        assert_eq!("my name", reg.try_get_value("root::name").unwrap());
        assert_eq!("123", reg.try_get_value("root::counter").unwrap());
        assert_eq!("234.567", reg.try_get_value("root::speed").unwrap());
        assert_eq!("false", reg.try_get_value("root::flag").unwrap());
        assert_eq!("220", reg.try_get_value("root::sub::speed").unwrap());

        reg.try_set_value("root::sub::speed", "5").unwrap();
        assert_eq!("5", reg.try_get_value("root::sub::speed").unwrap());

        let v = reg.complete("::s").unwrap();
        assert_eq!(v, ["root::speed", "root::sub"]);

        let v = reg.complete("::s::s").unwrap();
        assert_eq!(v, ["root::sub::speed"]);

        assert_eq!(
            Err(VarRegistryError::AlreadyExists),
            reg.add("root".to_owned(), &arc)
        );
    }

    #[test]
    fn var_registry_weak_refs() {
        let reg = VarRegistry::new();
        {
            let arc = wrap_var_bag(TestVars {
                counter: 123,
                flag: false,
                name: "my name".to_string(),
                speed: 234.567,
                sub: MoreTestVars { speed: 220.0 },
            });
            reg.add("root".to_owned(), &arc).unwrap();
            assert_eq!("my name", reg.try_get_value("root::name").unwrap());
            assert_eq!(
                Err(VarRegistryError::AlreadyExists),
                reg.add("root".to_owned(), &arc)
            );
        }
        let arc = wrap_var_bag(TestVars {
            counter: 123,
            flag: false,
            name: "new name".to_string(),
            speed: 234.567,
            sub: MoreTestVars { speed: 220.0 },
        });
        reg.add("root".to_owned(), &arc).unwrap();
        assert_eq!("new name", reg.try_get_value("root::name").unwrap());
        assert_eq!(
            Err(VarRegistryError::AlreadyExists),
            reg.add("root".to_owned(), &arc)
        );
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
        assert!(matches!(v, Variable::VarBag { .. }));
        let v = Variable::from(&c.sub.counter);
        assert!(matches!(v, Variable::Integer { .. }));
        let v = Variable::from(&c.sub);
        assert!(matches!(v, Variable::VarBag { .. }));
        let v = Variable::from(&c.sub.name);
        assert!(matches!(v, Variable::String { .. }));
        let v = Variable::from(&c.speed);
        assert!(matches!(v, Variable::Float { .. }));
        let v = Variable::from(&c.flag);
        assert!(matches!(v, Variable::Boolean { .. }));

        c.sub
            .counter
            .set_from_str(&mut empty_split(), "321")
            .unwrap();
        assert_eq!(c.sub.counter, 321);
        c.speed.set_from_str(&mut empty_split(), "3.33").unwrap();
        assert_eq!(c.speed, 3.33);
    }
}
