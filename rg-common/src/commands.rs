use std::{collections::HashMap, convert::Infallible, fmt::{Debug, Display}, slice::Iter, str::{FromStr, Split}};

///
/// 
/// 

#[derive(Default)]
pub struct CommandRegistry {
    data: HashMap<String, Box<dyn CommandWrapper>>
}

impl CommandRegistry {
    pub fn register1<A:FromStr + 'static>(&mut self, name:&str, handler: fn(a:A)->Result<(), CmdError>) -> Result<(), CmdError>{
        let h = Holder1 {
            handler: Box::new(handler)
        };
        let key = name.to_owned();
        if self.data.contains_key(&key) {
            return Err(CmdError::AlreadyExists(key));
        }
        self.data.insert(key, Box::new(h));
        //h.invoke(&mut s.split(" "))
        Ok(())
    }

    pub fn register2<A, B>(&mut self, name:&str, handler: fn(a:A,b:B)->Result<(), CmdError>) -> Result<(), CmdError>
    where A:FromStr + 'static, 
    B:FromStr + 'static
    {
        let h = Holder2 {
            handler: Box::new(handler)
        };
        let key = name.to_owned();
        if self.data.contains_key(&key) {
            return Err(CmdError::AlreadyExists(key));
        }
        self.data.insert(key, Box::new(h));
        //h.invoke(&mut s.split(" "))
        Ok(())
    }

    pub fn invoke(&self, name: String, args: &[String]) -> Result<(), CmdError> {
        if let Some(wrapper) = self.data.get(&name) {
            wrapper.invoke(args.iter())
        } else {
            Err(CmdError::NotFound(name))
        }
    }
}

trait CommandWrapper {

    fn invoke(&self, args: Iter<String>) -> Result<(), CmdError>;

}

#[derive(Debug)]
struct Holder1<A:FromStr> {
    handler: Box<fn(A) -> Result<(), CmdError>>
}

#[derive(Debug)]
struct Holder2<A:FromStr, B:FromStr> {
    handler: Box<fn(A,B) -> Result<(), CmdError>>
}

fn parse<T:FromStr>(value:&str) -> Result<T, CmdError> {
    value.parse().map_err(|e| CmdError::ParseError(value.to_owned()))
}

impl<A:FromStr> CommandWrapper for Holder1<A> {
    fn invoke(&self, mut args: Iter<String>) -> Result<(), CmdError> {
        let arg1 = args.next().ok_or(CmdError::ArgNumberMismatch(1))?;
        if args.next().is_some() {
            return Err(CmdError::ArgNumberMismatch(1));
        }
        (self.handler)(parse(arg1)?)
    }
}

impl<A:FromStr, B:FromStr> CommandWrapper for Holder2<A,B> {
    fn invoke(&self, mut args: Iter<String>) -> Result<(), CmdError> {
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
    AlreadyExists(String),
    ParseError(String),
    ArgNumberMismatch(i8),
    NotFound(String)
}

impl std::error::Error for CmdError {}

impl Display for CmdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmdError::ParseError(s) => {
                write!(f, "Unable to parse \"{s}\"!")
            },
            CmdError::ArgNumberMismatch(n) => {
                write!(f, "Expected {n} arguments!")
            },
            CmdError::AlreadyExists(n) => {
                write!(f, "Name already registered: {n}")
            },
            CmdError::NotFound(c) => {
                write!(f, "No such command: \"{c}\"")
            },
        }
    }
}

#[cfg(test)]
mod test {
    use crate::CommandRegistry;


    #[test]
    fn commands(){
        let mut reg = CommandRegistry::default();
        let handler = |a:String| {
            println!("Called test handler with {a}");
            Ok(())
        };
        reg.register1("test", handler);
        reg.register1("2", |a:i32|{
            println!("Called with {a}");
            Ok(())
        });

        reg.register2("3", |a:i32,b:String|{
            println!("Called with {a} and {b}");
            Ok(())
        });

        reg.invoke("test".to_owned(), &["Hello".to_owned()]).unwrap();
        reg.invoke("2".to_owned(), &["321".to_owned()]).unwrap();
        reg.invoke("3".to_owned(), &["123".to_owned(), "Hello_World!".to_owned()]).unwrap();
    }
}