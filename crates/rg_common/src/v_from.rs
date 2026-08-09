use std::borrow::Cow;
use std::fmt::{Display, Formatter};

use crate::vars::Variable;
use crate::VarBag;

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
            Variable::None => {
                write!(f, "None")
            }
        }
    }
}

macro_rules! impl_from_int {
    ( $($t:ty),* ) => {
        $(
        impl From<&$t> for Variable<'_> {
            fn from(value: &$t) -> Self {
                Variable::Integer(*value as i64)
            }
        }

        impl From<&mut $t> for Variable<'_> {
            fn from(value: &mut $t) -> Self {
                Variable::Integer(*value as i64)
            }
        }
        )*
    };
}

impl_from_int!{i8, u8, i16, u16, i32, u32, i64, u64, usize}

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

type DynVarBagRef = dyn VarBag + Send + Sync;

impl<'a> From<&'a DynVarBagRef> for Variable<'a> {
    fn from(value: &'a DynVarBagRef) -> Self {
        Variable::VarBag(value)
    }
}


macro_rules! impl_from_float {
    ( $($t:ty),* ) => {
        $(
        impl From<&$t> for Variable<'_> {
            fn from(value: &$t) -> Self {
                Variable::Float(*value as f64)
            }
        }

        impl From<&mut $t> for Variable<'_> {
            fn from(value: &mut $t) -> Self {
                Variable::Float(*value as f64)
            }
        }
        )*
    };
}

impl_from_float!{f32, f64}

impl<'a> From<&'a Option<String>> for Variable<'a> {
    fn from(value: &'a Option<String>) -> Self {
        value
            .as_ref()
            .map(|v| Variable::from(v))
            .unwrap_or(Variable::None)
    }
}
