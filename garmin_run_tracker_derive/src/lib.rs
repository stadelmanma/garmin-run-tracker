//! Define procedural macro to process service config entries
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;
extern crate proc_macro;
extern crate proc_macro2;

use proc_macro::TokenStream;
use syn::DeriveInput;

mod config;

#[proc_macro_derive(FromServiceConfig, attributes(service_config))]
pub fn derive_from_service_config(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    config::expand_derive_from_service_config(&mut input)
        .unwrap_or_else(to_compile_errors)
        .into()
}

fn to_compile_errors(errors: Vec<syn::Error>) -> proc_macro2::TokenStream {
    let compile_errors = errors.iter().map(syn::Error::to_compile_error);
    quote!(#(#compile_errors)*)
}
