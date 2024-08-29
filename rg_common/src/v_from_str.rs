use std::num::{ParseFloatError, ParseIntError};
use std::str::{ParseBoolError, Split};

use rg_common::VarBag;

use crate::vars::FromStrMutator;
use crate::VariableError;

///
/// Error converters
///
impl From<ParseIntError> for VariableError {
    fn from(value: ParseIntError) -> Self {
        VariableError::ParsingError
    }
}

impl From<ParseFloatError> for VariableError {
    fn from(value: ParseFloatError) -> Self {
        VariableError::ParsingError
    }
}

impl From<ParseBoolError> for VariableError {
    fn from(value: ParseBoolError) -> Self {
        VariableError::ParsingError
    }
}

///
/// Mutators
///
impl FromStrMutator for i32 {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
        assert!(sp.next().is_none());
        *self = value.parse::<i32>()?;
        Ok(())
    }
}

impl FromStrMutator for i64 {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
        assert!(sp.next().is_none());
        *self = value.parse::<i64>()?;
        Ok(())
    }
}

impl FromStrMutator for usize {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
        assert!(sp.next().is_none());
        *self = value.parse::<usize>()?;
        Ok(())
    }
}

impl FromStrMutator for f32 {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
        assert!(sp.next().is_none());
        *self = value.parse::<f32>()?;
        Ok(())
    }
}

impl FromStrMutator for f64 {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
        assert!(sp.next().is_none());
        *self = value.parse::<f64>()?;
        Ok(())
    }
}

impl FromStrMutator for bool {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
        assert!(sp.next().is_none());
        *self = value.parse::<bool>()?;
        Ok(())
    }
}

impl FromStrMutator for String {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
        assert!(sp.next().is_none());
        *self = value.to_string();
        Ok(())
    }
}

impl FromStrMutator for Option<String> {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
        assert!(sp.next().is_none());
        *self = if "None" != value {
            Some(value.to_string())
        } else {
            None
        };
        Ok(())
    }
}

impl<T: VarBag> FromStrMutator for T {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
        self.try_set_var(sp, value)
    }
}
