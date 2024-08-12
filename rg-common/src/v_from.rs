use std::borrow::Cow;
use std::fmt::{Display, Formatter};

use crate::VarBag;
use crate::vars::Variable;

impl Display for Variable<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Variable::VarBag(_) => {
                write!(f, "VarBag{{...}}")
            }
            Variable::String(v) => {
                write!(f, "{v}")
            }
            Variable::Integer(v) => {
                write!(f, "{v}")
            }
            Variable::Float(v) => {
                write!(f, "{v}")
            }
            Variable::Boolean(v) => {
                write!(f, "{v}")
            }
        }
    }
}

impl From<&bool> for Variable<'_> {
    fn from(value: &bool) -> Self {
        Variable::Boolean(*value)
    }
}

impl From<&mut bool> for Variable<'_> {
    fn from(value: &mut bool) -> Self {
        Variable::Boolean(*value)
    }
}

impl From<&i64> for Variable<'_> {
    fn from(value: &i64) -> Self {
        Variable::Integer(*value)
    }
}

impl From<&mut i64> for Variable<'_> {
    fn from(value: &mut i64) -> Self {
        Variable::Integer(*value)
    }
}

impl From<&i32> for Variable<'_> {
    fn from(value: &i32) -> Self {
        Variable::Integer(*value as i64)
    }
}

impl From<&mut i32> for Variable<'_> {
    fn from(value: &mut i32) -> Self {
        Variable::Integer(*value as i64)
    }
}

impl<'a> From<&'a str> for Variable<'a> {
    fn from(value: &'a str) -> Self {
        Variable::String(Cow::from(value))
    }
}

impl<'a> From<&'a String> for Variable<'a> {
    fn from(value: &'a String) -> Self {
        Variable::String(Cow::from(value))
    }
}

impl<'a> From<&'a mut String> for Variable<'a> {
    fn from(value: &'a mut String) -> Self {
        Variable::String(Cow::from(value as &String))
    }
}

impl<'a, T: VarBag> From<&'a T> for Variable<'a> {
    fn from(value: &'a T) -> Self {
        Variable::VarBag(value)
    }
}

impl<'a, T: VarBag> From<&'a mut T> for Variable<'a> {
    fn from(value: &'a mut T) -> Self {
        Variable::VarBag(value)
    }
}

impl From<&f64> for Variable<'_> {
    fn from(value: &f64) -> Self {
        Variable::Float(*value)
    }
}

impl From<&mut f64> for Variable<'_> {
    fn from(value: &mut f64) -> Self {
        Variable::Float(*value)
    }
}

impl From<&f32> for Variable<'_> {
    fn from(value: &f32) -> Self {
        Variable::Float(*value as f64)
    }
}

impl From<&mut f32> for Variable<'_> {
    fn from(value: &mut f32) -> Self {
        Variable::Float(*value as f64)
    }
}