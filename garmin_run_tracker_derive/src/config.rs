use proc_macro2::TokenStream;
use quote::quote;

pub fn expand_derive_from_service_config(
    input: &mut syn::DeriveInput,
) -> Result<TokenStream, Vec<syn::Error>> {
    let name = &input.ident;
    let expanded = quote! {
        // The generated impl.
        impl FromServiceConfig for #name {
            fn from_config(config: &ServiceConfig) -> Result<Self, Error> {
                Ok(Default::default())
            }
        }
    };

    // Hand the output tokens back to the compiler.
    Ok(TokenStream::from(expanded))
}
