use std::str::Split;

use rg_common::VarBag;

use crate::VariableError;
use crate::vars::FromStrMutator;

impl FromStrMutator for i32 {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
        assert!(sp.next().is_none());
        *self = value.parse::<i32>().map_err(|v| VariableError::ParsingError)?;
        Ok(())
    }
}

impl FromStrMutator for i64 {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
        assert!(sp.next().is_none());
        *self = value.parse::<i64>().map_err(|v| VariableError::ParsingError)?;
        Ok(())
    }
}

impl FromStrMutator for usize {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
        assert!(sp.next().is_none());
        *self = value.parse::<usize>().map_err(|v| VariableError::ParsingError)?;
        Ok(())
    }
}

impl FromStrMutator for f32 {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
        assert!(sp.next().is_none());
        *self = value.parse::<f32>().map_err(|v| VariableError::ParsingError)?;
        Ok(())
    }
}

impl FromStrMutator for f64 {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
        assert!(sp.next().is_none());
        *self = value.parse::<f64>().map_err(|v| VariableError::ParsingError)?;
        Ok(())
    }
}

impl FromStrMutator for bool {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
        assert!(sp.next().is_none());
        *self = value.parse::<bool>().map_err(|v| VariableError::ParsingError)?;
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