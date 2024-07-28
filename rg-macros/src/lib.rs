use proc_macro::{TokenStream};
use syn::__private::quote::quote;
use syn::Data;

#[proc_macro_derive(VarBag)]
pub fn var_bag(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let struct_identifier = &input.ident;
    match &input.data {
        Data::Struct(syn::DataStruct { fields, .. }) => {
            let mut ts = quote!{
            };

            for field in fields {
                let identifier = field.ident.as_ref().unwrap();
                ts.extend(quote!{
                    result.insert(stringify!(#identifier).to_string());
                });
            }

            // let mut implementation = quote!{
            //     let mut hash_map = std::collections::HashMap::<String, String>::new();
            // };
            //
            // for field in fields {
            //     let identifier = field.ident.as_ref().unwrap();
            //     implementation.extend(quote!{
            //         hash_map.insert(stringify!(#identifier).to_string(), String::from(value.#identifier));
            //     });
            // }

            quote! {
                #[automatically_derived]
                impl common::VarBag for #struct_identifier {
                    fn get_names(&self, result: &mut std::collections::HashSet<String>) {
                        #ts
                    }

                    // fn from(value: #struct_identifier) -> Self {
                    //     #implementation
                    //
                    //     hash_map
                    // }
                }
            }
        }
        _ => unimplemented!()
    }.into()
}
