use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex, PoisonError, Weak},
};

use thiserror::Error;

use crate::cmd_parser::parse_command_line;

///
/// Command registry
///
pub trait CmdAdapter: Fn(&[&str]) -> Result<(), CmdError> + Send + Sync {}

impl<T> CmdAdapter for T where T: Fn(&[&str]) -> Result<(), CmdError> + Send + Sync {}

type CmdMap = HashMap<String, Weak<dyn CmdAdapter>>;

#[derive(Default, Debug)]
pub struct CommandRegistry(Mutex<CmdMap>);

impl CommandRegistry {
    /// Binds passed adapter to [name] until weak reference is upgradeble.
    pub fn register(&self, name: String, adapter: Weak<dyn CmdAdapter>) -> Result<(), CmdError> {
        let mut guard = self.0.lock()?;
        if let Some(v) = guard.get(&name) {
            if v.strong_count() > 0 {
                return Err(CmdError::AlreadyExists);
            }
        }
        guard.insert(name, adapter);
        Ok(())
    }

    /// Invokes command handler by name passed as first argument in [args]
    pub fn invoke(&self, args: &[&str]) -> Result<(), CmdError> {
        if args.len() < 1 {
            return arg_num_mismatch(1, 0);
        }
        let guard = self.0.lock()?;
        if let Some(adapter) = guard.get(args[0]).and_then(|weak| weak.upgrade()) {
            drop(guard);
            return (adapter)(&args[1..]);
        }
        Err(CmdError::NotFound)
    }

    /// Parses [command] script and invokes command handler for each found command
    pub fn execute<S>(&self, command: S) -> Result<(), CmdError>
    where
        S: AsRef<str>,
    {
        let mut str = command.as_ref();
        while let (rest, Some(args)) = parse_command_line(str) {
            self.invoke(&args[..])?;
            match rest {
                Some(s) => str = s,
                None => break,
            }
        }
        Ok(())
    }
}

///
/// Command registry error
///
#[derive(Debug, Error)]
pub enum CmdError {
    #[error("Command already exists")]
    AlreadyExists,
    #[error("Unable to parse: \"{0}\"")]
    ParseError(String),
    #[error("Expected {expected} arguments got {actual}")]
    ArgNumberMismatch { expected: usize, actual: usize },
    #[error("No such command")]
    NotFound,
    #[error("Lock poisoned")]
    LockPoisoned,
}

fn arg_num_mismatch(expected: usize, actual: usize) -> Result<(), CmdError> {
    Err(CmdError::ArgNumberMismatch { expected, actual })
}

impl<T> From<PoisonError<T>> for CmdError {
    fn from(_: PoisonError<T>) -> Self {
        CmdError::LockPoisoned
    }
}

pub struct CommandBuilder<'a> {
    registry: &'a CommandRegistry,
    handlers: Vec<Arc<dyn CmdAdapter>>,
}

#[allow(dead_code)]
pub struct CommandOwner(Vec<Arc<dyn CmdAdapter>>);

pub trait FromContext {
    type Output<'r>;

    fn to_arg<'a>(value: Option<&'a str>) -> Result<Self::Output<'a>, CmdError>;
}

#[inline(always)]
fn parse<T>(v: &str) -> Result<T, CmdError>
where
    T: FromStr,
{
    v.parse()
        .map_err(|_| CmdError::ParseError(format!("Failed to parse {v}")))
}

macro_rules! impl_from_context {
    ( $($t:ty),* ) => {
        $(  impl FromContext for $t
            {
                type Output<'r> = Self;

                fn to_arg<'a>(value: Option<&'a str>) -> Result<Self::Output<'a>, CmdError> {
                    let raw_str = value.ok_or_else(|| no_value())?;
                    parse(raw_str)
                }
            }
        ) *
    }
}

#[inline]
fn no_value() -> CmdError {
    CmdError::ParseError("No value!".to_owned())
}

impl<T> FromContext for Option<T>
where
    T: FromStr,
{
    type Output<'r> = Self;

    fn to_arg<'a>(value: Option<&'a str>) -> Result<Self::Output<'a>, CmdError> {
        Ok(if let Some(v) = value {
            Some(parse::<T>(v)?)
        } else {
            None
        })
    }
}

impl FromContext for &str {
    type Output<'r> = &'r str;

    fn to_arg<'a>(value: Option<&'a str>) -> Result<Self::Output<'a>, CmdError> {
        value.ok_or_else(|| no_value())
    }
}

impl_from_context! {u8, u16, u32, u64, usize, i8, i16, i32, i64, f32, f64, String, bool}

fn ensure_at_most(expected: usize, actual: usize) -> Result<(), CmdError> {
    if actual > expected {
        return arg_num_mismatch(expected, actual);
    }
    Ok(())
}

///
/// Command builder
///
impl CommandBuilder<'_> {
    pub fn new<'a>(registry: &'a CommandRegistry) -> CommandBuilder<'a> {
        CommandBuilder {
            registry,
            handlers: Vec::new(),
        }
    }

    /// Binds supplied command handler [adapter] to [name]
    pub fn add<A, Args>(&mut self, name: &str, adapter: A) -> Result<(), CmdError>
    where
        A: IntoAdapter<Args> + 'static,
        Args: 'static,
    {
        let a = Arc::new(adapter.to_adapter());
        self.registry
            .register(name.to_owned(), Arc::downgrade(&a) as _)?;
        self.handlers.push(a);
        Ok(())
    }

    /// Returns command owner - the structure wich holds strong references to command handlers.
    /// Handler is bound to name in registry as long as this struct is not dropped.
    pub fn build(self) -> CommandOwner {
        CommandOwner(self.handlers)
    }
}

pub trait IntoAdapter<Args> {
    fn to_adapter(self) -> impl CmdAdapter;
}

macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + count!($($xs)*));
}

macro_rules! impl_as_adapter {
    ($($t:ident),*) => {
        impl<Func, $($t),*> IntoAdapter<($($t,)*)> for Func
        where
            for <'a> Func: Fn($($t),*) -> Result<(), CmdError> +
                Fn($(<$t as FromContext>::Output<'a>),*)-> Result<(), CmdError> + Send + Sync + 'static,
            $(
                $t : FromContext + 'static,
            )*
        {
            fn to_adapter(self) -> impl CmdAdapter {
                const ARG_COUNT: usize = count!($($t),*);

                move |args: &[&str]| {
                    ensure_at_most(ARG_COUNT, args.len())?;
                    let mut k = 0usize;

                    (self)(
                        $({
                            let arg = args.get(k).map(|s| *s);
                            let arg = $t::to_arg(arg)?;
                            k += 1;
                            arg
                        },)*
                    )
                }
            }
        }
    };
}

impl_as_adapter!();
impl_as_adapter!(A);
impl_as_adapter!(A, B);
impl_as_adapter!(A, B, C);
impl_as_adapter!(A, B, C, D);
impl_as_adapter!(A, B, C, D, E);
impl_as_adapter!(A, B, C, D, E, F);

///
/// Tests
///
#[cfg(test)]
mod test {
    use std::{
        ops::Deref,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use super::*;

    fn invoke<const N: usize, R: Deref<Target = CommandRegistry>>(
        reg: R,
        args: [&str; N],
    ) -> Result<(), CmdError> {
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
            Err(CmdError::ParseError(_))
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
            }
            println!("Name is: {}", ac.lock().unwrap().name);
            Ok(())
        })
        .unwrap();
        let ac = Arc::clone(&arc);
        b.add("data", move |v: Option<i32>| {
            let mut guard = ac.lock().unwrap();
            if let Some(v) = v {
                guard.data = v;
            }
            println!("data={}", guard.data);
            Ok(())
        })
        .unwrap();
        let counter = Arc::new(Mutex::new(0));
        let cloned = Arc::clone(&counter);
        b.add("two_args", move |a: bool, b: &str| {
            println!("Passed: {a}, {b}, {}", cloned.lock().unwrap());
            *cloned.lock().unwrap() += 1;
            Ok(())
        })
        .unwrap();
        arc.lock().unwrap().commands = Some(b.build());
        invoke(&reg, ["name"]).unwrap();
        invoke(&reg, ["name", "Guffy"]).unwrap();
        assert_eq!("Guffy", arc.lock().unwrap().name);
        invoke(&reg, ["data"]).unwrap();
        assert_eq!(123, arc.lock().unwrap().data);
        invoke(&reg, ["data", "77"]).unwrap();
        assert_eq!(77, arc.lock().unwrap().data);
        invoke(&reg, ["two_args", "true", "Wohoaa!"]).unwrap();
    }
}
