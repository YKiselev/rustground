use proc_macro::TokenStream;

use crate::var_bag::define_var_bag;

mod var_bag;

#[proc_macro_derive(VarBag, attributes(transient))]
pub fn var_bag(input: TokenStream) -> TokenStream {
    define_var_bag(syn::parse_macro_input!(input as syn::DeriveInput))
}
