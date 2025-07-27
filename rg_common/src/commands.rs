use std::{
    any::Any,
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex, PoisonError, Weak},
};

use snafu::Snafu;

///
///
///
type CmdAdapter = dyn Fn(&[String]) -> Result<(), CmdError>;
type CmdMap = HashMap<String, Weak<CmdAdapter>>;

#[derive(Default)]
pub struct CommandRegistry(Mutex<CmdMap>);

impl CommandRegistry {
    pub fn register<S>(&self, name: S, adapter: Weak<CmdAdapter>) -> Result<(), CmdError>
    where
        S: AsRef<str>,
    {
        let mut guard = self.0.lock()?;
        if let Some(v) = guard.get(name.as_ref()) {
            if v.strong_count() > 0 {
                return Err(CmdError::AlreadyExists);
            }
        }
        guard.insert(name.as_ref().to_owned(), adapter);
        Ok(())
    }

    pub fn invoke(&self, args: &[String]) -> Result<(), CmdError> {
        if args.len() < 1 {
            return arg_num_mismatch(1, 0);
        }
        let guard = self.0.lock()?;
        if let Some(adapter) = guard.get(&args[0]).and_then(|weak| weak.upgrade()) {
            drop(guard);
            return (adapter)(&args[1..]);
        }
        Err(CmdError::NotFound)
    }
}

///
/// Command registry error
///
#[derive(Debug, Snafu)]
pub enum CmdError {
    #[snafu(display("Command already exists"))]
    AlreadyExists,
    #[snafu(display("Unable to parse: \"{value}\""))]
    ParseError { value: String },
    #[snafu(display("Expected {expected} arguments got {actual}"))]
    ArgNumberMismatch { expected: usize, actual: usize },
    #[snafu(display("No such command"))]
    NotFound,
    #[snafu(display("Lock poisoned"))]
    LockPoisoned,
    #[snafu(display("Argument conversion failed"))]
    ConversionFailed,
}

fn arg_num_mismatch(expected: usize, actual: usize) -> Result<(), CmdError> {
    Err(CmdError::ArgNumberMismatch { expected, actual })
}

impl<T> From<PoisonError<T>> for CmdError {
    fn from(_: PoisonError<T>) -> Self {
        CmdError::LockPoisoned
    }
}

///
/// Command builder
///
pub struct CommandBuilder<'a> {
    registry: &'a CommandRegistry,
    handlers: Vec<Arc<dyn Any>>,
}

pub struct CommandOwner(Vec<Arc<dyn Any>>);

pub trait ToArg<Output = Self> {
    fn to_arg(value: Option<&str>) -> Result<Output, CmdError>;
}

#[inline(always)]
fn parse<T>(v: &str) -> Result<T, CmdError>
where
    T: FromStr,
{
    v.parse().map_err(|_| CmdError::ParseError {
        value: v.to_owned(),
    })
}

macro_rules! impl_to_arg {
    ( $($t:ty),* ) => {
        $(  impl ToArg for $t
            {
                fn to_arg(value: Option<&str>) -> Result<Self, CmdError> {
                    parse(value.ok_or(CmdError::ParseError {
                        value: "No value!".to_owned(),
                    })?)
                }
            }
        ) *
    }
}

impl<T> ToArg for Option<T>
where
    T: FromStr,
{
    fn to_arg(value: Option<&str>) -> Result<Self, CmdError> {
        Ok(match value {
            Some(v) => Some(parse::<T>(v)?),
            None => None,
        })
    }
}

impl_to_arg! {u8, u16, u32, u64, usize, i8, i16, i32, i64, f32, f64, String, bool}


fn check_args(expected: usize, actual: usize) -> Result<(), CmdError> {
    if actual > expected {
        return arg_num_mismatch(expected, actual);
    }
    Ok(())
}

fn adapter0<F>(handler: F) -> Box<CmdAdapter>
where
    F: Fn() -> Result<(), CmdError> + 'static,
{
    Box::new(move |args: &[String]| {
        check_args(0, args.len())?;
        (handler)()
    })
}

fn adapter1<F, A>(handler: F) -> Box<CmdAdapter>
where
    F: Fn(A) -> Result<(), CmdError> + 'static,
    A: ToArg<A>,
{
    Box::new(move |args: &[String]| {
        check_args(1, args.len())?;
        let arg1 = A::to_arg(args.get(0).map(|v| v.as_str()))?;
        (handler)(arg1)
    })
}

fn adapter2<F, A, B>(handler: F) -> Box<CmdAdapter>
where
    F: Fn(A, B) -> Result<(), CmdError> + 'static,
    A: ToArg<A>,
    B: ToArg<B>,
{
    Box::new(move |args: &[String]| {
        check_args(2, args.len())?;
        let arg1 = A::to_arg(args.get(0).map(|v| v.as_str()))?;
        let arg2 = B::to_arg(args.get(1).map(|v| v.as_str()))?;
        (handler)(arg1, arg2)
    })
}

fn adapter3<F, A, B, C>(handler: F) -> Box<CmdAdapter>
where
    F: Fn(A, B, C) -> Result<(), CmdError> + 'static,
    A: ToArg<A>,
    B: ToArg<B>,
    C: ToArg<C>,
{
    Box::new(move |args: &[String]| {
        check_args(3, args.len())?;
        let arg1 = A::to_arg(args.get(0).map(|v| v.as_str()))?;
        let arg2 = B::to_arg(args.get(1).map(|v| v.as_str()))?;
        let arg3 = C::to_arg(args.get(2).map(|v| v.as_str()))?;
        (handler)(arg1, arg2, arg3)
    })
}

impl CommandBuilder<'_> {
    pub fn new<'a>(registry: &'a CommandRegistry) -> CommandBuilder<'a> {
        CommandBuilder {
            registry,
            handlers: Vec::new(),
        }
    }

    fn add(&mut self, name: &str, adapter: Box<CmdAdapter>) -> Result<(), CmdError> {
        let a = Arc::new(adapter);
        self.registry.register(name, Arc::downgrade(&a) as _)?;
        self.handlers.push(a);
        Ok(())
    }

    pub fn add0<F>(&mut self, name: &str, handler: F) -> Result<(), CmdError>
    where
        F: Fn() -> Result<(), CmdError> + 'static,
    {
        self.add(name, adapter0(handler))
    }

    pub fn add1<A, F>(&mut self, name: &str, handler: F) -> Result<(), CmdError>
    where
        F: Fn(A) -> Result<(), CmdError> + 'static,
        A: ToArg,
    {
        self.add(name, adapter1(handler))
    }

    pub fn add2<A, B, F>(&mut self, name: &str, handler: F) -> Result<(), CmdError>
    where
        F: Fn(A, B) -> Result<(), CmdError> + 'static,
        A: ToArg,
        B: ToArg,
    {
        self.add(name, adapter2(handler))
    }

    pub fn add3<A, B, C, F>(&mut self, name: &str, handler: F) -> Result<(), CmdError>
    where
        F: Fn(A, B, C) -> Result<(), CmdError> + 'static,
        A: ToArg,
        B: ToArg,
        C: ToArg,
    {
        self.add(name, adapter3(handler))
    }

    pub fn build(self) -> CommandOwner {
        CommandOwner(self.handlers)
    }
}

///
/// Tests
///
#[cfg(test)]
mod test {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };

    use crate::{commands::CmdError, CommandRegistry};

    use super::{CommandBuilder, CommandOwner};

    fn invoke<const N: usize>(reg: &CommandRegistry, args: [&str; N]) -> Result<(), CmdError> {
        let args: Vec<String> = args.iter().map(|v| v.to_string()).collect();
        reg.invoke(args.as_slice())
    }

    fn build_and_invoke(reg: &CommandRegistry) {
        let mut b = CommandBuilder::new(reg);
        b.add0("1", || Ok(())).unwrap();
        b.add0("2", || Ok(())).unwrap();
        b.add0("3", || Ok(())).unwrap();
        let _cmds = b.build();
        invoke(&reg, ["1"]).unwrap();
        invoke(&reg, ["2"]).unwrap();
        invoke(&reg, ["3"]).unwrap();
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
        b.add0("0", || Ok(())).unwrap();
        b.add1("1", |a: i32| Ok(())).unwrap();
        assert!(matches!(
            b.add1("1", |a: String| Ok(())),
            Err(CmdError::AlreadyExists)
        ));
        b.add2("2", |a: i32, b: String| Ok(())).unwrap();
        b.add2("2_2", |a: i32, b: Option<String>| {
            print!("Got a={a}, b={b:?}");
            Ok(())
        })
        .unwrap();
        b.add3("3", |a: i32, b: u8, c: String| Ok(())).unwrap();
        let _cmds = b.build();

        invoke(&reg, ["0"]).unwrap();
        invoke(&reg, ["1", "123"]).unwrap();
        invoke(&reg, ["2_2", "321"]).unwrap();
        invoke(&reg, ["3", "123", "22", "Hello_World!"]).unwrap();

        assert!(matches!(
            invoke(&reg, ["1", "2.3"]),
            Err(CmdError::ParseError { value: _ })
        ));
        assert!(matches!(
            invoke(&reg, ["1", "2", ".3"]),
            Err(CmdError::ArgNumberMismatch {
                expected: 1,
                actual: 2
            })
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
        })
        .unwrap();
        let c3 = Arc::clone(&counter);
        b.add1("2", move |a: usize| {
            c3.fetch_add(a, Ordering::SeqCst);
            Ok(())
        })
        .unwrap();
        invoke(&reg, ["1", "5"]).unwrap();
        assert_eq!(15, counter.load(Ordering::Acquire));
    }

    struct Module {
        commands: Option<CommandOwner>,
        data: i32,
        name: String,
    }

    #[test]
    fn real_module() {
        let reg = CommandRegistry::default();
        let mut b = CommandBuilder::new(&reg);
        let mut arc = Arc::new(Mutex::new(Module {
            commands: None,
            data: 123,
            name: "Dummy".to_owned(),
        }));
        let m = Module {
            commands: None,
            data: 123,
            name: "one".to_owned(),
        };
        let ac = Arc::clone(&arc);
        b.add1("name", move |n: Option<String>| {
            if let Some(n) = n {
                ac.lock().unwrap().name = n;
            } else {
                println!("Name is: {}", ac.lock().unwrap().name);
            }
            Ok(())
        })
        .unwrap();
        arc.lock().unwrap().commands = Some(b.build());
        invoke(&reg, ["name"]).unwrap();
        invoke(&reg, ["name", "Guffy"]).unwrap();
        assert_eq!("Guffy", arc.lock().unwrap().name);
    }
}
