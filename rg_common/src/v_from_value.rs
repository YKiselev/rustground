use serde::Deserialize;
use toml::Value;

use crate::{vars::FromValue, VarBag, VariableError};

impl<T> FromValue for T
where
    T: VarBag + for<'de> Deserialize<'de>,
{
    fn from_value(value: Value) -> Result<Self, VariableError> {
        value
            .try_into()
            .map_err(|e| e.into())
    }
}

impl<T> FromValue for Option<T>
where
    T: FromValue + for<'de> Deserialize<'de>,
{
    fn from_value(value: Value) -> Result<Self, VariableError> {
        value
            .try_into()
            .map_err(|e| e.into())
    }
}

macro_rules! impl_from_value {
    ( $($t:ty),* ) => {
        $(
        impl FromValue for $t
        {
            fn from_value(value: Value) -> Result<Self, VariableError> {
                value.try_into()
                .map_err(|e| e.into())
            }
        }
        )*
    };
}

impl_from_value! {i32, i64, u32, u64, usize, f32, f64, bool, String}

#[cfg(test)]
mod tests {
    use toml::Table;

    use super::*;

    #[test]
    fn test_from_value() {
        let table: Table = toml::from_str("a = 123").unwrap();
        let av = i32::from_value(table.get("a").unwrap().clone()).unwrap();
        assert_eq!(123, av);
    }
}
