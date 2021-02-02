use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{Data, Fields};

pub fn expand_derive_from_service_config(
    input: &mut syn::DeriveInput,
) -> Result<TokenStream, Vec<syn::Error>> {
    let name = &input.ident;
    let setters = config_setters(&input.data);
    let expanded = quote! {
        // The generated impl
        impl FromServiceConfig for #name {
            fn from_config(config: &ServiceConfig) -> Result<Self, Error> {
                let mut base = Self::default();
                for key in config.parameters() {
                    match key.as_ref() {
                        #setters
                        _ => log::warn!(
                            "unknown configuration parameter for {}: {}={:?}",
                            stringify!(#name),
                            key,
                            config.get_parameter(key)
                        ),
                    }
                }
                Ok(base)
            }
        }
    };

    // Hand the output tokens back to the compiler.
    Ok(TokenStream::from(expanded))
}

/// Generate a setter method for each field that isn't annotated with #[service_config(skip)]
fn config_setters(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let mut recurse: Vec<_> = fields
                    .named
                    .iter()
                    .filter_map(|f| {
                        for attr in &f.attrs {
                            let attr_str = format!("{}", quote!(#attr));
                            if &attr_str == "#[service_config(skip)]" {
                                return None;
                            }
                        }
                        let name = f.ident.as_ref().unwrap();
                        let key = format!("{}", &name);
                        let ty = &f.ty;
                        let ty_str = format!("{}", ty.to_token_stream());
                        let tokens = match ty_str.as_ref() {
                            "String" => quote_spanned! {
                                f.span() => #key => {
                                    if let Some(val) = config.get_parameter_as_string(#key) {
                                        base.#name = val?
                                    }
                                }
                            },
                            "f32" | "f64" => quote_spanned! {
                                f.span() => #key => {
                                    if let Some(val) = config.get_parameter_as_f64(#key) {
                                        base.#name = val? as #ty
                                    }
                                }
                            },
                            "u8" | "u16" | "u32" | "u64" | "usize" => quote_spanned! {
                                f.span() => #key => {
                                    if let Some(val) = config.get_parameter_as_i64(#key) {
                                        base.#name = val? as #ty
                                    }
                                }
                            },
                            "i8" | "i16" | "i32" | "i64" | "isize" => quote_spanned! {
                                f.span() => #key => {
                                    if let Some(val) = config.get_parameter_as_i64(#key) {
                                        base.#name = val? as #ty
                                    }
                                }
                            },
                            _ => unimplemented!("Macro doesn't support type {} yet", ty_str),
                        };
                        Some(tokens)
                    })
                    .collect();

                if recurse.is_empty() {
                    recurse.push(quote! {"" => continue});
                }
                quote! {
                    #(#recurse),*,
                }
            }
            _ => unimplemented!("Only Fields::Named is supported"),
        },
        _ => unimplemented!("Only Data::Struct is supported"),
    }
}
