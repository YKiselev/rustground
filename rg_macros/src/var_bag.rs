use proc_macro::TokenStream;

use syn::__private::quote::quote;
use syn::{Attribute, Data, DeriveInput};

fn find_attribute<'a>(attrs: &'a Vec<Attribute>, path: &str) -> Option<&'a Attribute> {
    attrs.iter().find(|v| v.path().is_ident(path))
}

fn has_attribute(attrs: &Vec<Attribute>, path: &str) -> bool {
    find_attribute(attrs, path).is_some()
}

pub(crate) fn define_var_bag(input: DeriveInput) -> TokenStream {
    let struct_identifier = &input.ident;
    match &input.data {
        Data::Struct(syn::DataStruct { fields, .. }) => {
            let ids = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect::<Vec<_>>();
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

                    fn try_get_var(&self, name: &str) -> Option<rg_common::Variable<'_>> {
                        match name {
                            #(stringify!(#ids) => Some(rg_common::Variable::from(&self.#ids)),)*
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
                }
            }
        }
        _ => unimplemented!()
    }.into()
}
