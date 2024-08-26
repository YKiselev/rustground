use std::{
    any::Any,
    borrow::Borrow,
    cell::RefCell,
    collections::HashMap,
    fmt::Display,
    marker::PhantomData,
    str::FromStr,
    sync::{Arc, Mutex, MutexGuard, PoisonError, Weak},
};

///
///
///

type CmdMap = HashMap<String, Weak<dyn CommandWrapper>>;

#[derive(Default)]
pub struct CommandRegistry {
    data: Mutex<CmdMap>,
}

impl CommandRegistry {
    pub fn register(&self, name: &str, wrapper: Weak<dyn CommandWrapper>) -> Result<(), CmdError> {
        let mut guard = self.data.lock()?;
        if let Some(v) = guard.get(name) {
            if v.strong_count() > 0 {
                return Err(CmdError::AlreadyExists);
            }
        }
        guard.insert(name.to_owned(), wrapper);
        Ok(())
    }

    pub fn invoke(&self, args: Vec<String>) -> Result<(), CmdError> {
        if args.len() < 1 {
            return Err(CmdError::ArgNumberMismatch(1));
        }
        let guard = self.data.lock()?;
        if let Some(wrapper) = guard.get(&args[0]).and_then(|weak| weak.upgrade()) {
            drop(guard);
            return wrapper.invoke(&args[1..]);
        }
        Err(CmdError::NotFound)
    }
}

pub trait CommandWrapper {
    fn invoke(&self, args: &[String]) -> Result<(), CmdError>;
}

struct Holder {
    handler: Box<dyn Fn(&[String]) -> Result<(), CmdError>>,
}

struct Holder1<A: FromStr + 'static> {
    handler: Box<dyn Fn(A) -> Result<(), CmdError>>,
}

struct Holder2<A: FromStr, B: FromStr> {
    handler: Box<dyn Fn(A, B) -> Result<(), CmdError>>,
}

fn parse<T: FromStr>(value: &str) -> Result<T, CmdError> {
    value
        .parse()
        .map_err(|e| CmdError::ParseError(value.to_owned()))
}

impl CommandWrapper for Holder {
    fn invoke(&self, args: &[String]) -> Result<(), CmdError> {
        (self.handler)(args)
    }
}

impl<A: FromStr> CommandWrapper for Holder1<A> {
    fn invoke(&self, args: &[String]) -> Result<(), CmdError> {
        let mut it = args.into_iter();
        let arg1 = it.next().ok_or(CmdError::ArgNumberMismatch(1))?;
        if it.next().is_some() {
            return Err(CmdError::ArgNumberMismatch(1));
        }
        (self.handler)(parse(&arg1)?)
    }
}

impl<A: FromStr, B: FromStr> CommandWrapper for Holder2<A, B> {
    fn invoke(&self, args: &[String]) -> Result<(), CmdError> {
        let mut it = args.into_iter();
        let arg1 = it.next().ok_or(CmdError::ArgNumberMismatch(2))?;
        let arg2 = it.next().ok_or(CmdError::ArgNumberMismatch(2))?;
        if it.next().is_some() {
            return Err(CmdError::ArgNumberMismatch(2));
        }
        (self.handler)(parse(&arg1)?, parse(&arg2)?)
    }
}

///
/// Command registry error
///
#[derive(Debug)]
pub enum CmdError {
    AlreadyExists,
    ParseError(String),
    ArgNumberMismatch(i8),
    NotFound,
    LockPoisoned,
}

impl std::error::Error for CmdError {}

impl Display for CmdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmdError::ParseError(s) => {
                write!(f, "Unable to parse \"{s}\"!")
            }
            CmdError::ArgNumberMismatch(n) => {
                write!(f, "Expected {n} arguments!")
            }
            CmdError::AlreadyExists => {
                write!(f, "Name already registered!")
            }
            CmdError::NotFound => {
                write!(f, "No such command!")
            }
            CmdError::LockPoisoned => {
                write!(f, "Lock poisoned!")
            }
        }
    }
}

impl<T> From<PoisonError<T>> for CmdError {
    fn from(value: PoisonError<T>) -> Self {
        CmdError::LockPoisoned
    }
}

///
/// Command builder
///
pub struct CommandBuilder<'a> {
    registry: &'a CommandRegistry,
    handlers: Vec<Arc<dyn CommandWrapper>>,
}

pub struct CommandOwner {
    _handlers: Vec<Arc<dyn CommandWrapper>>,
}

impl CommandBuilder<'_> {
    pub fn new<'a>(registry: &'a CommandRegistry) -> CommandBuilder<'a> {
        CommandBuilder {
            registry,
            handlers: Vec::new(),
        }
    }

    pub fn add<F>(&mut self, name: &str, handler: F)
    where
        F: Fn(&[String]) -> Result<(), CmdError> + 'static,
    {
        self.try_add(name, handler).unwrap();
    }

    pub fn try_add<F>(&mut self, name: &str, handler: F) -> Result<(), CmdError>
    where
        F: Fn(&[String]) -> Result<(), CmdError> + 'static,
    {
        let h = Holder {
            handler: Box::new(handler),
        };
        let a = Arc::new(h);
        self.registry.register(name, Arc::downgrade(&a) as _)?;
        self.handlers.push(a);
        Ok(())
    }

    pub fn add1<A, F>(&mut self, name: &str, handler: F)
    where
        F: Fn(A) -> Result<(), CmdError> + 'static,
        A: FromStr + 'static,
    {
        self.try_add1(name, handler).unwrap();
    }

    pub fn try_add1<A, F>(&mut self, name: &str, handler: F) -> Result<(), CmdError>
    where
        F: Fn(A) -> Result<(), CmdError> + 'static,
        A: FromStr + 'static,
    {
        let h = Holder1 {
            handler: Box::new(handler),
        };
        let a = Arc::new(h);
        self.registry.register(name, Arc::downgrade(&a) as _)?;
        self.handlers.push(a);
        Ok(())
    }

    pub fn add2<A, B, F>(&mut self, name: &str, handler: F)
    where
        F: Fn(A, B) -> Result<(), CmdError> + 'static,
        A: FromStr + 'static,
        B: FromStr + 'static,
    {
        self.try_add2(name, handler).unwrap();
    }

    pub fn try_add2<A, B, F>(&mut self, name: &str, handler: F) -> Result<(), CmdError>
    where
        F: Fn(A, B) -> Result<(), CmdError> + 'static,
        A: FromStr + 'static,
        B: FromStr + 'static,
    {
        let h = Holder2 {
            handler: Box::new(handler),
        };
        let a = Arc::new(h);
        self.registry.register(name, Arc::downgrade(&a) as _)?;
        self.handlers.push(a);
        Ok(())
    }

    pub fn build(&mut self) -> CommandOwner {
        CommandOwner {
            _handlers: std::mem::take(&mut self.handlers),
        }
    }
}

///
/// Tests
///
#[cfg(test)]
mod test {
    use std::{
        collections::HashMap,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex, MutexGuard,
        },
    };

    use crate::{commands::CmdError, CommandRegistry};

    use super::{CmdMap, CommandBuilder};

    fn invoke<const N: usize>(reg: &CommandRegistry, args: [&str; N]) -> Result<(), CmdError> {
        reg.invoke(args.iter().map(|v| v.to_string()).collect())
    }

    fn build_and_invoke(reg: &CommandRegistry) {
        let mut b = CommandBuilder::new(reg);
        b.add("1", |a: &[String]| Ok(()));
        b.add("2", |a: &[String]| Ok(()));
        b.add("3", |a: &[String]| Ok(()));
        let _cmds = b.build();
        invoke(&reg, ["1", "Hello"]).unwrap();
        invoke(&reg, ["2", "Hello"]).unwrap();
        invoke(&reg, ["3", "Hello"]).unwrap();
    }

    #[test]
    fn lifetime() {
        let reg = CommandRegistry::default();
        build_and_invoke(&reg);
        {
            assert!(matches!(
                invoke(&reg, ["1", "2", ".3"]),
                Err(CmdError::NotFound)
            ));
            assert!(matches!(
                invoke(&reg, ["2", "2", ".3"]),
                Err(CmdError::NotFound)
            ));
            assert!(matches!(
                invoke(&reg, ["3", "2", ".3"]),
                Err(CmdError::NotFound)
            ));
        }
        build_and_invoke(&reg);
    }

    #[test]
    fn commands() {
        let reg = CommandRegistry::default();
        let mut b = CommandBuilder::new(&reg);
        b.add1("1", |a: String| Ok(()));
        assert!(matches!(
            b.try_add1("1", |a: i32| Ok(())),
            Err(CmdError::AlreadyExists)
        ));
        b.add1("2", |a: i32| Ok(()));
        b.add2("3", |a: i32, b: String| Ok(()));
        b.add("4", |a: &[String]| Ok(()));
        let _cmds = b.build();

        invoke(&reg, ["1", "Hello"]).unwrap();
        invoke(&reg, ["2", "321"]).unwrap();
        invoke(&reg, ["3", "123", "Hello_World!"]).unwrap();
        invoke(&reg, ["4", "1", "2"]).unwrap();
        assert!(matches!(
            invoke(&reg, ["2", "2.3"]),
            Err(CmdError::ParseError(_))
        ));
        assert!(matches!(
            invoke(&reg, ["2", "2", ".3"]),
            Err(CmdError::ArgNumberMismatch(1))
        ));
        assert!(matches!(
            invoke(&reg, ["nope", "2", ".3"]),
            Err(CmdError::NotFound)
        ));
    }

    #[test]
    fn recusrive() {
        let reg = Arc::new(CommandRegistry::default());
        let counter = Arc::new(AtomicUsize::default());
        let c2 = Arc::clone(&counter);
        let r2 = reg.clone();
        let mut b = CommandBuilder::new(reg.as_ref());
        b.add1("1", move |a: usize| {
            c2.fetch_add(a, Ordering::SeqCst);
            invoke(r2.as_ref(), ["2", &(a * 2).to_string()]).unwrap();
            Ok(())
        });
        let c3 = Arc::clone(&counter);
        b.add1("2", move |a: usize| {
            c3.fetch_add(a, Ordering::SeqCst);
            Ok(())
        });
        invoke(&reg, ["1", "5"]).unwrap();
        assert_eq!(15, counter.load(Ordering::Acquire));
    }
}
