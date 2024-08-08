use proc_macro::TokenStream;

use syn::{Attribute, Data, DeriveInput};
use syn::__private::quote::quote;

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
            let trans = fields.iter().map(|v| !has_attribute(&v.attrs, "transient")).collect::<Vec<_>>();
            quote! {
                #[automatically_derived]
                impl rg_common::VarBag for #struct_identifier {

                    fn get_vars(&self) -> std::vec::Vec<rg_common::VarInfo> {
                        let mut result = std::vec::Vec::new();
                        #(
                            result.push(rg_common::VarInfo {
                                name: stringify!(#ids),
                                persisted: #trans
                            });
                        )*
                        result
                    }

                    fn try_get_var(&self, name: &str) -> Option<rg_common::Variable<'_>> {
                        match name {
                            #(stringify!(#ids) => Some(rg_common::Variable::from(&self.#ids)),)*
                            _ => None
                        }
                    }

                    fn try_set_var(&mut self, name: &str, value: &str) -> Result<(), rg_common::VariableError> {
                        match name {
                            //#(stringify!(#ids) => { self.#ids = value.parse().map_err(|e| rg_common::VariableError::ParsingError)?; Ok(()) },)*
                            _ => Err(rg_common::VariableError::NotFound)
                        }
                    }
                }
            }
        }
        _ => unimplemented!()
    }.into()
}