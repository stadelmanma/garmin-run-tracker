use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{Data, Field, Fields, Type};

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
                        if skip_field(&f) {
                            None
                        } else {
                            Some(generate_setter(f))
                        }
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

fn skip_field(field: &Field) -> bool {
    for attr in &field.attrs {
        let attr_str = format!("{}", quote!(#attr));
        if &attr_str == "#[service_config(skip)]" {
            return true;
        }
    }
    false
}

fn generate_setter(field: &Field) -> TokenStream {
    let name = field.ident.as_ref().unwrap();
    let key = format!("{}", &name);
    let (get_fn, cast) = get_param_fn_ident(&field.ty);

    // generate assignment tokens w/wo casting type
    let assignment = if let Some(cast) = cast {
        quote_spanned! { field.span() =>  base.#name = val? as #cast }
    } else {
        quote_spanned! { field.span() => base.#name = val? }
    };

    // wrap assignment op with function to fetch value from config
    quote_spanned! {
        field.span() => #key => {
            if let Some(val) = config.#get_fn(#key) {
                #assignment
            }
        }
    }
}

fn get_param_fn_ident(ty: &Type) -> (Ident, Option<&Type>) {
    let type_str = format!("{}", ty.to_token_stream());
    let cast = Some(ty);
    match type_str.as_ref() {
        "String" => (format_ident!("{}", "get_parameter_as_string"), None),
        "f32" | "f64" => (format_ident!("{}", "get_parameter_as_f64"), cast),
        "u8" | "u16" | "u32" | "u64" | "usize" => {
            (format_ident!("{}", "get_parameter_as_i64"), cast)
        }
        "i8" | "i16" | "i32" | "i64" | "isize" => {
            (format_ident!("{}", "get_parameter_as_i64"), cast)
        }
        _ => unimplemented!("Macro doesn't support type {}", type_str),
    }
}
