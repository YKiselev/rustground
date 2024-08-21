use std::{
    borrow::Borrow, cell::RefCell, collections::HashMap, fmt::Display, ops::Deref, rc::{Rc, Weak}, str::FromStr, sync::{Arc, Mutex, MutexGuard, PoisonError}
};

///
///
///

type CmdMap = HashMap<String, Box<dyn CommandWrapper>>;

#[derive(Default)]
pub struct CommandRegistry {
    data: Arc<Mutex<CmdMap>>,
}

struct ThreadLocalGuard<'a> {
    guard: Rc<MutexGuard<'a, CmdMap>>,
}

impl<'a> ThreadLocalGuard<'a> {
    std::thread_local! {
        static GUARD: RefCell<Weak<MutexGuard<'static, CmdMap>>> = RefCell::new(Weak::new());
    }

    fn new(data: &'static Arc<Mutex<CmdMap>>) -> Self {
        let guard = if let Some(existing) = Self::GUARD.with(|cell| cell.borrow().upgrade()) {
            existing
        } else {
            let rc = Rc::new(data.lock().unwrap());
            Self::GUARD.set(Rc::downgrade(&rc));
            rc
        };
        ThreadLocalGuard { guard }
    }
}

impl<'a> Deref for ThreadLocalGuard<'a> {
    type Target = MutexGuard<'a, CmdMap>;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl CommandRegistry {
    fn register_wrapper(
        &mut self,
        name: &str,
        wrapper: Box<dyn CommandWrapper>,
    ) -> Result<(), CmdError> {
        let mut guard = self.data.lock()?;
        if guard.contains_key(name) {
            return Err(CmdError::AlreadyExists);
        }
        guard.insert(name.to_owned(), wrapper);
        Ok(())
    }

    pub fn register(
        &mut self,
        name: &str,
        handler: &'static dyn Fn(&[String]) -> Result<(), CmdError>
    ) -> Result<(), CmdError> {
        let h = Holder {
            handler: Box::new(handler)
        };
        self.register_wrapper(name, Box::new(h))
    }

    pub fn register1<A: FromStr + 'static>(
        &mut self,
        name: &str,
        handler: impl Fn(A) -> Result<(), CmdError> + 'static,
    ) -> Result<(), CmdError> {
        let h = Holder1 {
            handler: Box::new(handler)
        };
        self.register_wrapper(name, Box::new(h))
    }

    pub fn register2<A, B>(
        &mut self,
        name: &str,
        handler: fn(a: A, b: B) -> Result<(), CmdError>,
    ) -> Result<(), CmdError>
    where
        A: FromStr + 'static,
        B: FromStr + 'static,
    {
        let h = Holder2 {
            handler: Box::new(handler),
        };
        self.register_wrapper(name, Box::new(h))
    }

    pub fn invoke(&self, args: Vec<String>) -> Result<(), CmdError> {
        if args.len() < 1 {
            return Err(CmdError::ArgNumberMismatch(1));
        }
        let guard = self.data.lock()?;
        if let Some(wrapper) = guard.get(&args[0]) {
            wrapper.invoke(&args[1..])
        } else {
            Err(CmdError::NotFound)
        }
    }
}

trait CommandWrapper {
    fn invoke(&self, args: &[String]) -> Result<(), CmdError>;
}

struct Holder {
    handler: Box<dyn Fn(&[String]) -> Result<(), CmdError>>,
}

struct Holder1<A: FromStr + 'static> {
    handler: Box<dyn Fn(A) -> Result<(), CmdError>>,
}

struct Holder2<A: FromStr, B: FromStr> {
    handler: Box<fn(A, B) -> Result<(), CmdError>>,
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
            return Err(CmdError::ArgNumberMismatch(1));
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

#[cfg(test)]
mod test {
    use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};

    use crate::{commands::CmdError, CommandRegistry};

    fn invoke<const N: usize>(reg: &CommandRegistry, args: [&str; N]) -> Result<(), CmdError> {
        reg.invoke(args.iter().map(|v| v.to_string()).collect())
    }

    #[test]
    fn commands() {
        let mut reg = CommandRegistry::default();
        reg.register1("1", &|a: String| Ok(()));
        assert!(matches!(
            reg.register1("1", &|a: i32| { Ok(()) }),
            Err(CmdError::AlreadyExists)
        ));
        reg.register1("2", &|a: i32| Ok(()));

        reg.register2("3", |a: i32, b: String| Ok(()));
        reg.register("4", &|a: &[String]| Ok(()));

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
    fn threading() {
        let mut reg = CommandRegistry::default();
        let counter = Arc::new(AtomicUsize::default());
        let c2 = Arc::clone(&counter);
        reg.register1("1", move |a: usize| {
            c2.fetch_add(a, Ordering::SeqCst);
            Ok(())
        });
        invoke(&reg, ["1", "5"]).unwrap();
        assert_eq!(5, counter.load(Ordering::Acquire));
    }
}
