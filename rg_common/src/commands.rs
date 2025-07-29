use std::{
    any::Any,
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex, PoisonError, Weak},
};

use snafu::Snafu;

use crate::cmd_parser::parse_command_line;

///
/// Command registry
///
pub trait CmdAdapter: Fn(&[String]) -> Result<(), CmdError> + Send + Sync {}

impl<T> CmdAdapter for T where T: Fn(&[String]) -> Result<(), CmdError> + Send + Sync {}

type CmdMap = HashMap<String, Weak<dyn CmdAdapter>>;

#[derive(Default)]
pub struct CommandRegistry(Mutex<CmdMap>);

impl CommandRegistry {
    pub fn register<S>(&self, name: S, adapter: Weak<dyn CmdAdapter>) -> Result<(), CmdError>
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

    pub fn execute<S>(&self, command: S) -> Result<(), CmdError>
    where
        S: AsRef<str>,
    {
        let mut chars = command.as_ref().chars();
        loop {
            match parse_command_line(&mut chars) {
                Some(ref args) => self.invoke(args)?,
                None => break,
            }
        }
        Ok(())
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
    handlers: Vec<Arc<dyn CmdAdapter>>,
}

pub struct CommandOwner(Vec<Arc<dyn CmdAdapter>>);

pub trait FromContext<Output = Self> {
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
        $(  impl FromContext for $t
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

impl<T> FromContext for Option<T>
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

fn adapter0<F>(handler: F) -> impl CmdAdapter
where
    F: Fn() -> Result<(), CmdError> + Send + Sync + 'static,
{
    move |args: &[String]| {
        check_args(0, args.len())?;
        (handler)()
    }
}

fn adapter1<F, A>(handler: F) -> impl CmdAdapter
where
    F: Fn(A) -> Result<(), CmdError> + Send + Sync + 'static,
    A: FromContext<A>,
{
    move |args: &[String]| {
        check_args(1, args.len())?;
        let arg1 = A::to_arg(args.get(0).map(|v| v.as_str()))?;
        (handler)(arg1)
    }
}

fn adapter2<F, A, B>(handler: F) -> impl CmdAdapter
where
    F: Fn(A, B) -> Result<(), CmdError> + Send + Sync + 'static,
    A: FromContext<A>,
    B: FromContext<B>,
{
    move |args: &[String]| {
        check_args(2, args.len())?;
        let arg1 = A::to_arg(args.get(0).map(|v| v.as_str()))?;
        let arg2 = B::to_arg(args.get(1).map(|v| v.as_str()))?;
        (handler)(arg1, arg2)
    }
}

fn adapter3<F, A, B, C>(handler: F) -> impl CmdAdapter
where
    F: Fn(A, B, C) -> Result<(), CmdError> + Send + Sync + 'static,
    A: FromContext<A>,
    B: FromContext<B>,
    C: FromContext<C>,
{
    move |args: &[String]| {
        check_args(3, args.len())?;
        let arg1 = A::to_arg(args.get(0).map(|v| v.as_str()))?;
        let arg2 = B::to_arg(args.get(1).map(|v| v.as_str()))?;
        let arg3 = C::to_arg(args.get(2).map(|v| v.as_str()))?;
        (handler)(arg1, arg2, arg3)
    }
}

impl CommandBuilder<'_> {
    pub fn new<'a>(registry: &'a CommandRegistry) -> CommandBuilder<'a> {
        CommandBuilder {
            registry,
            handlers: Vec::new(),
        }
    }

    pub fn add<A, Args>(&mut self, name: &str, adapter: A) -> Result<(), CmdError>
    where
        A: AsAdapter<Args> + 'static,
        Args: 'static,
    {
        let a = Arc::new(adapter.as_handler());
        self.registry.register(name, Arc::downgrade(&a) as _)?;
        self.handlers.push(a);
        Ok(())
    }

    pub fn build(self) -> CommandOwner {
        CommandOwner(self.handlers)
    }
}

pub trait AsAdapter<Args> {
    fn as_handler(self) -> impl CmdAdapter;
}

impl<F> AsAdapter<()> for F
where
    F: Fn() -> Result<(), CmdError> + Send + Sync + 'static,
{
    fn as_handler(self) -> impl CmdAdapter {
        adapter0(self)
    }
}

impl<F, A> AsAdapter<(A,)> for F
where
    F: Fn(A) -> Result<(), CmdError> + Send + Sync + 'static,
    A: FromContext + 'static,
{
    fn as_handler(self) -> impl CmdAdapter {
        adapter1(self)
    }
}

impl<F, A, B> AsAdapter<(A, B)> for F
where
    F: Fn(A, B) -> Result<(), CmdError> + Send + Sync + 'static,
    A: FromContext + 'static,
    B: FromContext + 'static,
{
    fn as_handler(self) -> impl CmdAdapter {
        adapter2(self)
    }
}

impl<F, A, B, C> AsAdapter<(A, B, C)> for F
where
    F: Fn(A, B, C) -> Result<(), CmdError> + Send + Sync + 'static,
    A: FromContext + 'static,
    B: FromContext + 'static,
    C: FromContext + 'static,
{
    fn as_handler(self) -> impl CmdAdapter {
        adapter3(self)
    }
}

///
/// Tests
///
#[cfg(test)]
mod test {
    use std::{
        ops::Deref,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        },
    };

    use crate::{commands::CmdError, CommandRegistry};

    use super::{CommandBuilder, CommandOwner};

    fn invoke<const N: usize, R: Deref<Target = CommandRegistry>>(
        reg: R,
        args: [&str; N],
    ) -> Result<(), CmdError> {
        let args: Vec<String> = args.iter().map(|v| v.to_string()).collect();
        reg.invoke(args.as_slice())
    }

    fn build_and_invoke(reg: &CommandRegistry) {
        let mut b = CommandBuilder::new(reg);
        b.add("1", || Ok(())).unwrap();
        b.add("2", || Ok(())).unwrap();
        b.add("3", || Ok(())).unwrap();
        let _cmds = b.build();
        invoke(reg, ["1"]).unwrap();
        invoke(reg, ["2"]).unwrap();
        invoke(reg, ["3"]).unwrap();
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
        b.add("0", || Ok(())).unwrap();
        b.add("1", |a: i32| Ok(())).unwrap();
        assert!(matches!(
            b.add("1", |a: String| Ok(())),
            Err(CmdError::AlreadyExists)
        ));
        b.add("2", |a: i32, b: String| Ok(())).unwrap();
        b.add("2_2", |a: i32, b: Option<String>| {
            print!("Got a={a}, b={b:?}");
            Ok(())
        })
        .unwrap();
        b.add("3", |a: i32, b: u8, c: String| Ok(())).unwrap();
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
        let r2 = Arc::clone(&reg);
        let mut b = CommandBuilder::new(reg.as_ref());
        b.add("1", move |a: usize| {
            c2.fetch_add(a, Ordering::SeqCst);
            invoke(r2.clone(), ["2", &(a * 2).to_string()]).unwrap();
            Ok(())
        })
        .unwrap();
        let c3 = Arc::clone(&counter);
        b.add("2", move |a: usize| {
            c3.fetch_add(a, Ordering::SeqCst);
            Ok(())
        })
        .unwrap();
        invoke(reg, ["1", "5"]).unwrap();
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
        let arc = Arc::new(Mutex::new(Module {
            commands: None,
            data: 123,
            name: "Dummy".to_owned(),
        }));
        let ac = Arc::clone(&arc);
        b.add("name", move |n: Option<String>| {
            if let Some(n) = n {
                ac.lock().unwrap().name = n;
            } else {
                println!("Name is: {}", ac.lock().unwrap().name);
            }
            Ok(())
        })
        .unwrap();
        b.add("wow", || {
            println!("It works!");
            Ok(())
        })
        .unwrap();
        b.add("wow2", |a: i32| {
            println!("It works: {a}");
            Ok(())
        })
        .unwrap();
        b.add("wow3", |a: bool, b: String| {
            println!("It works: {a},{b}");
            Ok(())
        })
        .unwrap();
        arc.lock().unwrap().commands = Some(b.build());
        invoke(&reg, ["name"]).unwrap();
        invoke(&reg, ["name", "Guffy"]).unwrap();
        assert_eq!("Guffy", arc.lock().unwrap().name);
        invoke(&reg, ["wow"]).unwrap();
        invoke(&reg, ["wow2", "777"]).unwrap();
        invoke(&reg, ["wow3", "true", "Wohoaa!"]).unwrap();
    }
}
