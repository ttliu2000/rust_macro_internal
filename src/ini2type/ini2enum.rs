use proc_macro::TokenStream;
use syn::{ItemEnum, LitInt, parse_macro_input};

use parser_lib::ini::*;

use crate::utils::*;

use syn::{LitStr, Ident};
use syn::{
    parse::{Parse, ParseStream},
    Result, Token,
};

struct IniEnumArgs {
    path: LitStr,
    repr: Option<Ident>,
}

impl Parse for IniEnumArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let path: LitStr = input.parse()?;

        let mut repr = None;

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;

            let key: Ident = input.parse()?;
            if key == "repr" {
                input.parse::<Token![=]>()?;
                repr = Some(input.parse()?);
            } else {
                return Err(input.error("expected `repr = <type>`"));
            }
        }

        Ok(Self { path, repr })
    }
}

fn generate_item_to_string_fn(
    enum_name: &Ident,
    variants: &Vec<syn::Variant>,
) -> proc_macro2::TokenStream {
    let match_arms = variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        let variant_name_str = LitStr::new(&variant_ident.to_string(), variant_ident.span());
        quote::quote! {
            #enum_name::#variant_ident => #variant_name_str,
        }
    });

    quote::quote! {
        impl #enum_name {
            pub fn item_to_string(&self) -> &'static str {
                match self {
                    #(#match_arms)*
                }
            }
        }
    }
}


/// Generate an enum from an INI file
/// /// Usage:
/// ```ignore
/// ini_enum("path/to/file.ini", repr = u8);
/// ```
/// The INI file should contain key-value pairs where keys are used as enum variant names
/// and values are used as their corresponding integer values.
pub fn expand(attr: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as IniEnumArgs);
    let path = match get_file_pathbuf(&args.path) {
        Ok(n) => n,
        Err(e) => return e,
    };

    let mut s = parse_macro_input!(input as ItemEnum);
    let enum_name = &s.ident;

    if let Some(repr_ident) = args.repr {
        let repr_attr: syn::Attribute = syn::parse_quote!(
            #[repr(#repr_ident)]
        );

        // Avoid duplicate repr
        let has_repr = s.attrs.iter().any(|a| a.path().is_ident("repr"));
        if !has_repr {
            s.attrs.push(repr_attr);
        }
    }

    // add allow non_camel_case_types attribute to allow non-camel case variant names
    let allow_non_camel_case_types_attr: syn::Attribute = syn::parse_quote!(
        #[allow(non_camel_case_types)]
    );
    s.attrs.push(allow_non_camel_case_types_attr);

    let doc_str = format!("Struct items generated from file '{}'", path.display());
    let doc = create_doc_comment(&doc_str);
    s.attrs.push(doc);
    
    match get_ini_location_and_properties(&path.display().to_string()) {
        Ok((_file_name, v)) => {
            v.iter().for_each(|p| {
                let variant_name = syn::Ident::new(&p.get_name().to_uppercase(), enum_name.span());
                let value = LitInt::new(p.get_value(), proc_macro2::Span::call_site());
                s.variants.push(syn::Variant {
                    attrs: vec![],
                    ident: variant_name,
                    fields: syn::Fields::Unit,
                    discriminant: Some((
                        syn::token::Eq::default(),
                        syn::parse_quote!(#value),
                    )),
                });
            });

            let vars = s.variants.iter().cloned().collect::<Vec<_>>();
            let item_to_string_impl = generate_item_to_string_fn(enum_name, &vars);

            quote::quote!
            {
                #s

                #item_to_string_impl
            }.into()
        }
        Err(e) => {
            let file_path = path.display().to_string();
            let err_msg = format!("failed to parse/process INI file '{}': '{:?}'", file_path, e);
            to_error(&err_msg)
        }
    }
}
