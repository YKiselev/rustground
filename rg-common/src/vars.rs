use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::sync::atomic::AtomicBool;

pub trait VarBag {
    fn get_names(&self) -> HashSet<String>;

    //fn get(name: &str) -> Option<String>;

    //fn set(name: &str) -> Result<(), NoSuchVariableError>;
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

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd, Copy, Clone)]
struct NoSuchVariableError;

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
    use std::fmt::Display;

    use rg_macros::VarBag;

    use crate::VarBag;
    use crate::vars::Vars;

    #[derive(VarBag, Default)]
    pub(crate) struct TestVars {
        pub(crate) counter: i32,
        pub(crate) flag: bool,
        pub(crate) name: String,
    }

    #[test]
    fn var_bag() {
        let v = TestVars::default();
        let names = v.get_names();
        assert_eq!(names, ["counter", "flag", "name"].iter().map(|e| e.to_string()).collect());
    }

    #[test]
    fn register<'a>() {
        let mut vars = Vars::default();
        /*
                vars.put("int", 123i32);
                vars.put("long", 123i64);
                vars.put("float", 123f32);
                vars.put("double", 123f64);
                vars.put("string", "hello");
                println!("Map is {:?}", vars.vars);
                println!("int={:?}", vars.get::<i32>("int"));
                println!("long={:?}", vars.get::<i64>("long"));
                println!("float={:?}", vars.get::<f32>("float"));
                println!("double={:?}", vars.get::<f64>("double"));
                //println!("string={:?}", vars.get::<String>("string"));
                let res = vars.inspect("string", |v| {
                    if let Value::STRING(s) = v {
                        println!("String is \"{}\"", s);
                    };
                });*/
    }
}