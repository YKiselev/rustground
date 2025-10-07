use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::convert::Infallible;
use std::fmt::{Debug};
use std::iter::Peekable;
use std::ops::Deref;
use std::str::Split;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard, Weak};

use thiserror::Error;
use toml::ser::Buffer;
use toml::{Table, Value};

pub enum Variable<'a> {
    VarBag(&'a dyn VarBag),
    String(Cow<'a, str>),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    None,
}

impl<'a> Variable<'a> {
    pub fn try_get_var(self, sp: &mut Split<&str>) -> Option<Variable<'a>> {
        match &self {
            Variable::VarBag(bag) => bag.try_get_var(sp),
            _ => {
                if sp.next().is_some() {
                    None
                } else {
                    Some(self)
                }
            }
        }
    }
}

pub trait VarBag: erased_serde::Serialize {
    fn get_vars(&self) -> Vec<String>;

    fn try_get_var(&self, sp: &mut Split<&str>) -> Option<Variable<'_>>;

    fn try_set_var(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError>;

    fn populate(&mut self, value: Value) -> Result<(), VariableError>;
}

erased_serde::serialize_trait_object!(VarBag);

pub trait FromStrMutator {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError>;
}

pub trait FromValue: Sized {
    fn from_value(value: Value) -> Result<Self, VariableError>;
}

type VarBagBox = dyn VarBag + Send + Sync;
type VarBagRef = Weak<RwLock<VarBagBox>>;
type VarBagMap = HashMap<String, VarBagRef>;

#[derive(Debug, Default)]
struct InnerData {
    vars: VarBagMap,
    table: Table,
}

#[derive(Debug, Default)]
pub struct VarRegistry(RwLock<InnerData>);

impl VarRegistry {
    pub const DELIMITER: &'static str = "::";

    pub fn new(table: Option<Table>) -> Self {
        if table.is_some() {
            Self(RwLock::new(InnerData {
                table: table.unwrap(),
                ..Default::default()
            }))
        } else {
            Self::default()
        }
    }

    pub fn set_table(&self, table: Table) -> Result<(), VarRegistryError> {
        let mut guard = self.write().ok_or(VarRegistryError::LockFailed)?;
        guard.table = table;
        for (name, part) in guard.vars.iter() {
            if let Some(arc) = part.upgrade() {
                sync_with_table(&guard.table, name.as_ref(), &arc)?;
            }
        }
        Ok(())
    }

    pub fn to_toml(&self) -> Result<String, VarRegistryError> {
        let guard = self.read().ok_or(VarRegistryError::LockFailed)?;
        let mut buf = Buffer::new();
        let _ = erased_serde::serialize(&guard.vars, toml::Serializer::pretty(&mut buf))
            .map_err(|e| VarRegistryError::TomlError(e))?;
        Ok(buf.to_string())
    }

    pub fn add<S, T>(&self, name: S, part: &Arc<RwLock<T>>) -> Result<(), VarRegistryError>
    where
        S: Into<String> + AsRef<str>,
        T: VarBag + Send + Sync + 'static,
    {
        let mut guard = self.write().ok_or(VarRegistryError::LockFailed)?;
        let vb: Arc<RwLock<VarBagBox>> = part.clone();
        sync_with_table(&guard.table, name.as_ref(), &vb)?;
        match guard.vars.entry(name.into()) {
            Entry::Occupied(mut entry) => {
                if entry.get().upgrade().is_some() {
                    Err(VarRegistryError::AlreadyExists)
                } else {
                    entry.insert(Arc::downgrade(&vb));
                    Ok(())
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(Arc::downgrade(&vb));
                Ok(())
            }
        }
    }

    fn read(&self) -> Option<RwLockReadGuard<'_, InnerData>> {
        self.0.read().ok()
    }

    fn write(&self) -> Option<RwLockWriteGuard<'_, InnerData>> {
        self.0.write().ok()
    }

    pub fn try_get_value<S>(&self, name: S) -> Option<String>
    where
        S: AsRef<str>,
    {
        let mut sp = name.as_ref().split(Self::DELIMITER);
        let guard = self.read()?;
        let arc = guard.vars.get(sp.next()?)?.upgrade()?;
        let v_guard = arc.read().ok()?;
        let v = Variable::from(v_guard.deref()).try_get_var(&mut sp)?;

        match v {
            Variable::VarBag(_) => None,
            Variable::String(s) => {
                return if sp.next().is_none() {
                    Some(s.to_string())
                } else {
                    None
                };
            }
            Variable::Integer(i) => {
                if sp.next().is_none() {
                    Some(i.to_string())
                } else {
                    None
                }
            }
            Variable::Float(f) => {
                if sp.next().is_none() {
                    Some(f.to_string())
                } else {
                    None
                }
            }
            Variable::Boolean(b) => {
                if sp.next().is_none() {
                    Some(b.to_string())
                } else {
                    None
                }
            }
            Variable::None => {
                if sp.next().is_none() {
                    Some("None".to_string())
                } else {
                    None
                }
            }
        }
    }

    pub fn try_set_value(&self, name: &str, value: &str) -> Result<(), VarRegistryError> {
        let mut sp = name.split(Self::DELIMITER);
        let guard = self.read().ok_or(VarRegistryError::LockFailed)?;
        let key = sp.next().ok_or(not_found())?;
        let arc = guard
            .vars
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
        let guard = self.read()?;
        let mut result = Vec::new();
        for (key, value) in guard.vars.iter() {
            if !bag_name.is_empty() && !key.starts_with(bag_name) {
                continue;
            }
            if let Some(arc) = value.upgrade() {
                if let Ok(lr) = arc.read() {
                    let start = result.len();
                    filter_names(lr.deref(), &mut sp, "", &mut result);
                    for v in result[start..].iter_mut() {
                        *v = key.clone() + VarRegistry::DELIMITER + v;
                    }
                }
            }
        }
        Some(result)
    }
}

fn sync_with_table(
    table: &Table,
    name: &str,
    part: &Arc<RwLock<VarBagBox>>,
) -> Result<(), VarRegistryError> {
    if let Some(sub_table) = table.get(name) {
        let mut guard = part.write().map_err(|_| VarRegistryError::LockFailed)?;
        guard.populate(sub_table.clone())?;
    }
    Ok(())
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
            if let Some(v) = owner.try_get_var(&mut var_name.split(VarRegistry::DELIMITER)) {
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

#[derive(Debug, Error)]
pub enum VarRegistryError {
    #[error(transparent)]
    VarError(#[from] VariableError),
    #[error("Already esists")]
    AlreadyExists,
    #[error("Lock failed")]
    LockFailed,
    #[error(transparent)]
    TomlError(#[from] toml::ser::Error),
}

#[derive(Debug, Clone, Error)]
pub enum VariableError {
    #[error(transparent)]
    IntParsingError(#[from] std::num::ParseIntError),
    #[error(transparent)]
    FloatParsingError(#[from] std::num::ParseFloatError),
    #[error(transparent)]
    BoolParsingError(#[from] std::str::ParseBoolError),
    #[error(transparent)]
    DeserializationError(#[from] toml::de::Error),
    #[error("Not found")]
    NotFound,
    #[error("Expected Table got {0:?}")]
    TableExpected(toml::Value),
}

impl From<Infallible> for VariableError {
    fn from(_: Infallible) -> Self {
        VariableError::NotFound
    }
}

pub fn wrap_var_bag<T>(value: T) -> Arc<RwLock<T>>
where
    T: VarBag + Send + Sync + 'static,
{
    Arc::new(RwLock::new(value))
}

#[cfg(test)]
mod test {
    use std::fmt::Debug;
    use std::str::Split;

    use rg_macros::VarBag;
    use serde::Deserialize;
    use serde::Serialize;

    use super::*;

    #[derive(VarBag, Default, Serialize, Deserialize)]
    struct TestVars {
        counter: i32,
        flag: bool,
        name: String,
        speed: f64,
        #[serde(default)]
        sub: MoreTestVars,
    }

    #[derive(VarBag, Default, Serialize, Deserialize)]
    struct MoreTestVars {
        speed: f32,
        #[serde(default)]
        deep: DeepOne,
    }

    #[derive(VarBag, Default, Serialize, Deserialize)]
    struct DeepOne {
        key: String,
    }

    fn sp(value: &str) -> Split<'_, &str> {
        value.split(VarRegistry::DELIMITER)
    }

    #[test]
    fn var_bag() {
        let mut v = TestVars {
            flag: false,
            counter: 123,
            name: "some name".to_string(),
            speed: 345.466,
            sub: MoreTestVars {
                speed: 330.0,
                deep: DeepOne {
                    key: "()".to_owned(),
                },
            },
        };

        assert_eq!("false", v.try_get_var(&mut sp("flag")).unwrap().to_string());
        assert_eq!(
            "123",
            v.try_get_var(&mut sp("counter")).unwrap().to_string()
        );
        assert_eq!(
            "some name",
            v.try_get_var(&mut sp("name")).unwrap().to_string()
        );
        assert!(v.try_get_var(&mut sp("unknown")).is_none());

        v.try_set_var(&mut sp("flag"), "true").unwrap();
        v.try_set_var(&mut sp("name"), "New name").unwrap();
        v.try_set_var(&mut sp("counter"), "321").unwrap();

        assert_eq!("true", v.try_get_var(&mut sp("flag")).unwrap().to_string());
        assert_eq!(
            "321",
            v.try_get_var(&mut sp("counter")).unwrap().to_string()
        );
        assert_eq!(
            "New name",
            v.try_get_var(&mut sp("name")).unwrap().to_string()
        );
    }

    #[test]
    fn var_bag_populate() {
        let mut v = TestVars {
            flag: false,
            counter: 123,
            name: "some name".to_string(),
            speed: 345.466,
            sub: MoreTestVars {
                speed: 330.0,
                ..Default::default()
            },
        };

        let value = toml::from_str::<Value>(
            r#"
        flag = true
        counter = 10
        name = "Void"
        speed = 1.5
        sub.speed = 8.5
        sub.deep.key = "Yep"
        "#,
        )
        .unwrap();
        v.populate(value).unwrap();

        assert_eq!("true", v.try_get_var(&mut sp("flag")).unwrap().to_string());
        assert_eq!("10", v.try_get_var(&mut sp("counter")).unwrap().to_string());
        assert_eq!("Void", v.try_get_var(&mut sp("name")).unwrap().to_string());
        assert_eq!("1.5", v.try_get_var(&mut sp("speed")).unwrap().to_string());
        assert_eq!(
            "8.5",
            v.try_get_var(&mut sp("sub::speed")).unwrap().to_string()
        );
    }

    #[test]
    fn var_registry() {
        let reg = VarRegistry::default();
        let arc = wrap_var_bag(TestVars {
            counter: 123,
            flag: false,
            name: "my name".to_string(),
            speed: 234.567,
            sub: MoreTestVars {
                speed: 220.0,
                ..Default::default()
            },
        });
        reg.add("root", &arc).unwrap();
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

        assert!(matches!(
            reg.add("root", &arc),
            Err(VarRegistryError::AlreadyExists)
        ));
    }

    #[test]
    fn var_registry_weak_refs() {
        let reg = VarRegistry::default();
        {
            let arc = wrap_var_bag(TestVars {
                counter: 123,
                flag: false,
                name: "my name".to_string(),
                speed: 234.567,
                sub: MoreTestVars {
                    speed: 220.0,
                    ..Default::default()
                },
            });
            reg.add("root".to_owned(), &arc).unwrap();
            assert_eq!("my name", reg.try_get_value("root::name").unwrap());
            assert!(matches!(
                reg.add("root", &arc),
                Err(VarRegistryError::AlreadyExists)
            ));
        }
        let arc = wrap_var_bag(TestVars {
            counter: 123,
            flag: false,
            name: "new name".to_string(),
            speed: 234.567,
            sub: MoreTestVars {
                speed: 220.0,
                deep: DeepOne {
                    key: "Key1".to_owned(),
                },
            },
        });
        reg.add("root", &arc).unwrap();
        assert_eq!("new name", reg.try_get_value("root::name").unwrap());
        assert!(matches!(
            reg.add("root", &arc),
            Err(VarRegistryError::AlreadyExists)
        ));
    }

    const TABLE: &str = r#"
[root]
counter = 777
flag = true
name = "from table!"
speed = 3.555

[root.sub]
speed = 110.5
        "#;

    #[test]
    fn var_registry_set_table_before_varbag() {
        let reg = VarRegistry::default();
        let arc = wrap_var_bag(TestVars {
            counter: 123,
            flag: false,
            name: "new name".to_string(),
            speed: 234.567,
            sub: MoreTestVars {
                speed: 220.0,
                ..Default::default()
            },
        });

        let table = toml::from_str(TABLE).unwrap();
        reg.set_table(table).unwrap();
        reg.add("root", &arc).unwrap();

        assert_eq!("777", reg.try_get_value("root::counter").unwrap());
        assert_eq!("true", reg.try_get_value("root::flag").unwrap());
        assert_eq!("from table!", reg.try_get_value("root::name").unwrap());
        assert_eq!("3.555", reg.try_get_value("root::speed").unwrap());
        assert_eq!("110.5", reg.try_get_value("root::sub::speed").unwrap());

        let toml = reg.to_toml().unwrap();
        println!("Registry table looks like that:\n{}\n===", toml);
    }

    #[test]
    fn var_registry_set_table_after_varbag() {
        let reg = VarRegistry::default();
        let arc = wrap_var_bag(TestVars {
            counter: 123,
            flag: false,
            name: "new name".to_string(),
            speed: 234.567,
            sub: MoreTestVars {
                speed: 220.0,
                ..Default::default()
            },
        });
        reg.add("root", &arc).unwrap();

        let table = toml::from_str(TABLE).unwrap();
        reg.set_table(table).unwrap();

        assert_eq!("777", reg.try_get_value("root::counter").unwrap());
        assert_eq!("true", reg.try_get_value("root::flag").unwrap());
        assert_eq!("from table!", reg.try_get_value("root::name").unwrap());
        assert_eq!("3.555", reg.try_get_value("root::speed").unwrap());
        assert_eq!("110.5", reg.try_get_value("root::sub::speed").unwrap());
    }

    #[derive(Debug, VarBag, Serialize, Deserialize)]
    struct Sub {
        name: String,
        counter: i32,
    }

    #[derive(VarBag, Serialize, Deserialize)]
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
