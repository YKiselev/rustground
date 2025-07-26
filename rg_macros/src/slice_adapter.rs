use proc_macro::TokenStream;
use syn::{Data, DeriveInput, Fields, Generics, Ident};

pub(crate) fn define_slice_adapter(input: DeriveInput) -> TokenStream {
    let name = &input.ident;
    let (impl_gen, type_gen, where_clause) = input.generics.split_for_impl();
    match &input.data {
        Data::Struct(syn::DataStruct { fields, .. }) => {
            let info = get_field_infos(name, fields);
            let helpers = define_helper_methods(name, &info, &input.generics);
            let adapter = define_slice_adapter_struct(name, &info);
            let quoted = quote::quote! {
                #helpers
                #adapter
            };
            TokenStream::from(quoted)
        }
        _ => unimplemented!(),
    }
}

fn get_field_infos(owner_name: &Ident, fields: &Fields) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            quote::quote! {
                (
                    stringify!(#field_name),
                    std::mem::offset_of!(#owner_name, #field_name),
                    Self::size_of(|s: #owner_name| s.#field_name )
                )
            }
        })
        .collect()
}

fn define_helper_methods(
    name: &Ident,
    info: &Vec<proc_macro2::TokenStream>,
    generics: &Generics,
) -> proc_macro2::TokenStream {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();
    let len = info.len();
    quote::quote! {
        #[automatically_derived]
        impl #impl_gen #name #type_gen #where_clause {
            pub(crate) fn get_field_info() -> [(&'static str, usize, usize); #len] {
                [
                    #(#info),*
                ]
            }

            fn size_of<F, T, U>(_: F) -> usize
            where
                F: FnOnce(T) -> U,
            {
                std::mem::size_of::<U>()
            }
        }
    }
}

fn define_slice_adapter_struct(
    name: &Ident,
    info: &Vec<proc_macro2::TokenStream>
) -> proc_macro2::TokenStream {
    let adapter_ident = quote::format_ident!("{}{}", name, "SliceAdapter");
    let len = info.len();
    quote::quote! {

        pub struct #adapter_ident<'a> {
            slice: &'a [u8]
        }

        impl<'a> #adapter_ident<'a> {
            pub fn new(slice:&'a [u8]) -> Self {
                Self {
                    slice
                }
            }
        }
    }
}
