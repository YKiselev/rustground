use std::num::{ParseFloatError, ParseIntError};
use std::ops::DerefMut;
use std::str::{FromStr, ParseBoolError, Split};

use rg_common::VarBag;

use crate::vars::FromStrMutator;
use crate::VariableError;

///
/// Error converters
///
macro_rules! impl_parsing_error_from {
    ( $($t:ty),* ) => {
        $(
            impl From<$t> for VariableError {
                fn from(_: $t) -> Self {
                    VariableError::ParsingError
                }
            }
        )*
    };
}

impl_parsing_error_from! { ParseIntError, ParseFloatError, ParseBoolError }

///
/// Mutators
///
macro_rules! impl_from_str_mutator {
    ( $($t:ty),* ) => {
        $(  impl FromStrMutator for $t
            {
                fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
                    if sp.next().is_some() {
                        return Err(VariableError::NotFound);
                    }
                    *self = value.parse::<$t>()?;
                    Ok(())
                }
            }
        )*
    }
}

impl_from_str_mutator! { i32, i64, u32, u64, usize, f32, f64, bool, String }

// impl FromStrMutator for String {
//     fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
//         if sp.next().is_some() {
//             return Err(VariableError::NotFound);
//         }
//         *self = value.to_string();
//         Ok(())
//     }
// }

// impl FromStrMutator for Option<String> {
//     fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
//         if sp.next().is_some() {
//             return Err(VariableError::NotFound);
//         }
//         *self = if "None" != value {
//             Some(value.to_string())
//         } else {
//             None
//         };
//         Ok(())
//     }
// }

impl<T: FromStr> FromStrMutator for Option<T> {
    fn set_from_str(&mut self, sp: &mut Split<&str>, value: &str) -> Result<(), VariableError> {
        if sp.next().is_some() {
            return Err(VariableError::NotFound);
        }
        *self = if "None" != value {
            Some(
                value
                    .parse::<T>()
                    .map_err(|_| VariableError::ParsingError)?,
            )
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
