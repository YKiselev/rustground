use std::{collections::HashMap, str::FromStr};

use toml::Value;

use crate::{FromValue, VarBag, Variable, VariableError};

impl<V> VarBag for HashMap<String, V>
where
    for<'a> V: serde::Deserialize<'a> + serde::Serialize + FromStr + FromValue,
    for<'a> Variable<'a>: From<&'a V>,
{
    fn get_vars(&self) -> std::vec::Vec<&str> {
        self.keys().map(|k| k.as_str()).collect()
    }

    fn try_get_var(&self, sp: &mut std::str::Split<&str>) -> Option<Variable<'_>> {
        let name = sp.next();
        if name.is_none() {
            return Some(Variable::from_map(self));
        }
        self.get(name.unwrap()).map(|v| Variable::from(v))
    }

    fn try_set_var(
        &mut self,
        sp: &mut std::str::Split<&str>,
        value: &str,
    ) -> Result<(), VariableError> {
        let part = sp.next().ok_or(VariableError::NotFound)?;
        let v = V::from_str(value).map_err(|_| VariableError::ParsingError)?;
        self.insert(part.to_string(), v)
            .ok_or_else(|| VariableError::NotFound)?;
        Ok(())
    }

    fn populate(&mut self, value: Value) -> Result<(), VariableError> {
        match value {
            Value::Table(map) => {
                for (k, v) in map {
                    if let Ok(v) = V::from_value(v) {
                        self.insert(k, v);
                    }
                }
                Ok(())
            }
            _ => Err(VariableError::TableExpected(value)),
        }
    }
}
