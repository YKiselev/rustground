use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::sync::atomic::AtomicBool;

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

// impl Display for Vars<'_> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         f.write_fmt(format_args!("{}", self.vars))
//     }
// }

#[cfg(test)]
mod test {
    use std::fmt::Display;

    use crate::vars::{Value, Vars};

    #[test]
    fn register<'a>() {
        let mut vars = Vars::default();
        // let mut var1 = Some(2);
        //
        // trait Val: Display {}
        //
        // let map: HashMap<Cow<'a, str>, Box<dyn Any>> = HashMap::new();
        // let arc = Arc::new(RwLock::new(map));
        // {
        //     let mut g = arc.write().expect("AAA");
        //     g.insert(Cow::from("a"), Box::new("B"));
        //     g.insert(Cow::from("b"), Box::new("C"));
        //     g.insert(Cow::from("c"), Box::new(123u128));
        // }
        // {
        //     let mut g = arc.write().expect("AAA2");
        //     g.insert(Cow::from("d"), Box::new(7.5));
        //     g.insert(Cow::from("e"), Box::new(true));
        // }
        // let mut g = arc.write().expect("AAA2");
        // println!("Map is {:?}", g);
        // let v = g.get_mut("c").expect("AAAa");
        // println!("Is u128? {}", v.is::<u128>());
        // let u128: &mut u128 = v.downcast_mut().expect("No value!");
        // *u128 = 33333;
        //
        // println!("Now c={:?}", v.downcast_ref::<u128>().expect("Oops!"));

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
        });
    }
}