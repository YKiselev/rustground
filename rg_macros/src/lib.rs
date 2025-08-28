use proc_macro::TokenStream;
use slice_adapter::define_slice_adapter;

use crate::var_bag::define_var_bag;

mod var_bag;
mod slice_adapter;

#[proc_macro_derive(VarBag)]
pub fn var_bag(input: TokenStream) -> TokenStream {
    define_var_bag(syn::parse_macro_input!(input as syn::DeriveInput))
}

// #[proc_macro_derive(SliceAdapter)]
// pub fn slice_adapter(input: TokenStream) -> TokenStream {
//     define_slice_adapter(syn::parse_macro_input!(input as syn::DeriveInput))
// }