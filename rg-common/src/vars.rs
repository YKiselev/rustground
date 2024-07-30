use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::sync::atomic::AtomicBool;

#[derive(Debug, Default)]
pub struct VarInfo {
    pub name: &'static str,
    pub persisted: bool,
}

pub trait VarBag {
    fn get_vars(&self) -> Vec<VarInfo>;

    fn try_get_var(&self, name: &str) -> Option<String>;

    fn try_set_var(&mut self, name: &str, value: &str) -> Result<(), NoSuchVariableError>;
}

type Key<'a> = Cow<'a, str>;

#[derive(Debug)]
enum Value {
    BOOL(Arc<AtomicBool>),
    STRING(Arc<RwLock<String>>),
}

#[derive(Default)]
pub struct Vars<'a> {
    vars: RwLock<HashMap<Key<'a>, Value>>,
}

#[derive(Debug, PartialEq, Eq)]
struct RegisterError;

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd, Copy, Clone, Default)]
pub struct NoSuchVariableError;

impl Vars<'_> {
    pub fn put<V: Into<Value>>(&mut self, name: &str, value: V) {
        self.vars.write().expect("Failed to obtain write lock!").insert(Cow::from(name.to_owned()), value.into());
    }

    pub fn get<V: for<'a> From<&'a Value>>(&self, name: &str) -> Option<V> {
        self.vars.read().expect("Unable to obtain read lock!").get(name).map(|v| V::from(v))
    }

    pub fn inspect<F: Fn(&Value) -> ()>(&self, name: &str, handler: F) -> Option<()> {
        self.vars.read()
            .expect("Unable to obtain read lock!")
            .get(name)
            .map(|v| handler(v))
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::fmt::Display;

    use rg_macros::VarBag;

    use crate::vars::VarBag;

    #[derive(VarBag, Default)]
    pub(crate) struct TestVars {
        pub(crate) counter: i32,
        #[transient]
        pub(crate) flag: bool,
        pub(crate) name: String,
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
        //dbg!(infos);

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
}