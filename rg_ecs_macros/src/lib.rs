use proc_macro::{TokenStream};
use syn::{parse_macro_input, ItemFn};
use syn::__private::quote::quote;

#[proc_macro]
pub fn system(input: TokenStream) -> TokenStream {
    let copy = input.clone();
    let parsed = parse_macro_input!(copy as ItemFn);

    input
}
