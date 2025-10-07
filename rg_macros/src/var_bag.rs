use proc_macro::TokenStream;

//use syn::__private::quote::quote;
use quote::quote;
use syn::{Data, DeriveInput};

pub(crate) fn define_var_bag(input: DeriveInput) -> TokenStream {
    let struct_identifier = &input.ident;
    match &input.data {
        Data::Struct(syn::DataStruct { fields, .. }) => {
            let ids = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect::<Vec<_>>();
            let types = fields.iter().map(|f| {
                let ty = &f.ty;
                quote!(#ty)
            }).collect::<Vec<_>>();
            quote! {
                #[automatically_derived]
                impl rg_common::VarBag for #struct_identifier {

                    fn get_vars(&self) -> std::vec::Vec<String> {
                        let mut result = std::vec::Vec::new();
                        #(
                            result.push(String::from(stringify!(#ids)));
                        )*
                        result
                    }

                    fn try_get_var(&self, sp: &mut std::str::Split<&str>) -> Option<rg_common::Variable<'_>> {
                        let name = sp.next();
                        if name.is_none() {
                            return Some(rg_common::Variable::from(self));
                        }
                        match name.unwrap() {
                            #(
                                stringify!(#ids) => rg_common::Variable::from(&self.#ids).try_get_var(sp),
                            )*
                            _ => None
                        }
                    }

                    fn try_set_var(&mut self, sp: &mut std::str::Split<&str>, value: &str) -> Result<(), rg_common::VariableError> {
                        use rg_common::FromStrMutator;

                        let part = sp.next().ok_or(rg_common::VariableError::NotFound)?;

                        match part {
                            #(stringify!(#ids) => {
                                self.#ids.set_from_str(sp, value)?;
                                Ok(())
                            },)*
                            _ => Err(rg_common::VariableError::NotFound)
                        }
                    }

                    fn populate(&mut self, value: toml::Value) -> Result<(), rg_common::VariableError> {
                        use toml::Value;
                        use rg_common::FromValue;

                        match value {
                            Value::Table(map) => {
                                #(
                                if let Some(v) = map.get(stringify!(#ids)) {
                                    self.#ids = <#types as FromValue>::from_value(v.clone())?;
                                }
                                )*
                                Ok(())
                            },
                            _ => Err(rg_common::VariableError::TableExpected(value))
                        }
                    }
                }
            }
        }
        _ => unimplemented!()
    }.into()
}
