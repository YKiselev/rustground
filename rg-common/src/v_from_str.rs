use rg_common::VarBag;

use crate::VariableError;
use crate::vars::FromStrMutator;

impl FromStrMutator for i32 {
    fn set_from_str(&mut self, value: &str) -> Result<(), VariableError> {
        *self = value.parse::<i32>().map_err(|v| VariableError::ParsingError)?;
        Ok(())
    }
}

impl FromStrMutator for i64 {
    fn set_from_str(&mut self, value: &str) -> Result<(), VariableError> {
        *self = value.parse::<i64>().map_err(|v| VariableError::ParsingError)?;
        Ok(())
    }
}

impl FromStrMutator for f32 {
    fn set_from_str(&mut self, value: &str) -> Result<(), VariableError> {
        *self = value.parse::<f32>().map_err(|v| VariableError::ParsingError)?;
        Ok(())
    }
}

impl FromStrMutator for f64 {
    fn set_from_str(&mut self, value: &str) -> Result<(), VariableError> {
        *self = value.parse::<f64>().map_err(|v| VariableError::ParsingError)?;
        Ok(())
    }
}

impl FromStrMutator for bool {
    fn set_from_str(&mut self, value: &str) -> Result<(), VariableError> {
        *self = value.parse::<bool>().map_err(|v| VariableError::ParsingError)?;
        Ok(())
    }
}

impl FromStrMutator for String {
    fn set_from_str(&mut self, value: &str) -> Result<(), VariableError> {
        *self = value.to_string();
        Ok(())
    }
}

impl<T: VarBag> FromStrMutator for T {
    fn set_from_str(&mut self, value: &str) -> Result<(), VariableError> {
        Err(VariableError::ParsingError)
    }
}