use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    slice::Iter,
    str::FromStr,
};

///
///
///

#[derive(Default)]
pub struct CommandRegistry {
    data: HashMap<String, Box<dyn CommandWrapper>>,
}

impl CommandRegistry {
    pub fn register1<A: FromStr + 'static>(
        &mut self,
        name: &str,
        handler: fn(a: A) -> Result<(), CmdError>,
    ) -> Result<(), CmdError> {
        if self.data.contains_key(name) {
            return Err(CmdError::AlreadyExists);
        }
        let h = Holder1 {
            handler: Box::new(handler),
        };
        self.data.insert(name.to_owned(), Box::new(h));
        Ok(())
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
        if self.data.contains_key(name) {
            return Err(CmdError::AlreadyExists);
        }
        let h = Holder2 {
            handler: Box::new(handler),
        };
        self.data.insert(name.to_owned(), Box::new(h));
        Ok(())
    }

    pub fn invoke(&self, args: &mut Iter<&str>) -> Result<(), CmdError> {
        let name = args.next().ok_or(CmdError::ArgNumberMismatch(1))?;
        if let Some(wrapper) = self.data.get(*name) {
            wrapper.invoke(args)
        } else {
            Err(CmdError::NotFound)
        }
    }
}

trait CommandWrapper {
    fn invoke(&self, args: &mut Iter<&str>) -> Result<(), CmdError>;
}

#[derive(Debug)]
struct Holder1<A: FromStr> {
    handler: Box<fn(A) -> Result<(), CmdError>>,
}

#[derive(Debug)]
struct Holder2<A: FromStr, B: FromStr> {
    handler: Box<fn(A, B) -> Result<(), CmdError>>,
}

fn parse<T: FromStr>(value: &str) -> Result<T, CmdError> {
    value
        .parse()
        .map_err(|e| CmdError::ParseError(value.to_owned()))
}

impl<A: FromStr> CommandWrapper for Holder1<A> {
    fn invoke(&self, args: &mut Iter<&str>) -> Result<(), CmdError> {
        let arg1 = args.next().ok_or(CmdError::ArgNumberMismatch(1))?;
        if args.next().is_some() {
            return Err(CmdError::ArgNumberMismatch(1));
        }
        (self.handler)(parse(arg1)?)
    }
}

impl<A: FromStr, B: FromStr> CommandWrapper for Holder2<A, B> {
    fn invoke(&self, args: &mut Iter<&str>) -> Result<(), CmdError> {
        let arg1 = args.next().ok_or(CmdError::ArgNumberMismatch(2))?;
        let arg2 = args.next().ok_or(CmdError::ArgNumberMismatch(2))?;
        if args.next().is_some() {
            return Err(CmdError::ArgNumberMismatch(1));
        }
        (self.handler)(parse(arg1)?, parse(arg2)?)
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
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{commands::CmdError, CommandRegistry};

    #[test]
    fn commands() {
        let mut reg = CommandRegistry::default();
        reg.register1("1", |a: String| {
            println!("Called test handler with {a}");
            Ok(())
        });
        assert!(matches!(reg.register1("1", |a:i32|{ Ok(())}), Err(CmdError::AlreadyExists)));
        reg.register1("2", |a: i32| {
            println!("Called with {a}");
            Ok(())
        });

        reg.register2("3", |a: i32, b: String| {
            println!("Called with {a} and {b}");
            Ok(())
        });

        reg.invoke( &mut ["1","Hello"].iter())
            .unwrap();
        reg.invoke( &mut ["2","321"].iter()).unwrap();
        reg.invoke(
            &mut ["3","123", "Hello_World!"].iter(),
        )
        .unwrap();
        assert!(matches!(reg.invoke( &mut ["2","2.3"].iter()), Err(CmdError::ParseError(_))));
        assert!(matches!(reg.invoke( &mut ["2", "2", ".3"].iter()), Err(CmdError::ArgNumberMismatch(1))));
    }
}
