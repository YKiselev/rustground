use proc_macro::{TokenStream};
use syn::__private::quote::quote;
use syn::Data;

#[proc_macro_derive(VarBag)]
pub fn var_bag(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let struct_identifier = &input.ident;
    match &input.data {
        Data::Struct(syn::DataStruct { fields, .. }) => {
            let field_identifiers = fields.iter()
                .map(|f| f.ident.as_ref().unwrap())
                .collect::<Vec<_>>();

            quote! {
                #[automatically_derived]
                impl rg_common::VarBag for #struct_identifier {
                    fn get_names(&self) -> std::collections::HashSet<String> {
                        let mut result = std::collections::HashSet::new();
                        #(
                            result.insert(stringify!(#field_identifiers).to_string());
                        )*
                        result
                    }
                }
            }
        }
        _ => unimplemented!()
    }.into()
}
