use crate::init_args::InitArgs;
use crate::utils::*;
use parser_lib::ini::*;
use syn::LitStr;

fn get_enum_variants(v:&Vec<Property>, item_enum: &syn::ItemEnum) -> proc_macro2::TokenStream {
    let mut variants = vec![];

    for p in v {
        let variant_name = to_rust_enum_variant_name(&p.get_name());
        let variant_ident = match make_ident(&variant_name, item_enum.ident.span()) {
            Ok(id) => id,
            Err(e) => return e.into(),
        };

        let variant_name = format!("{} in ini file", p.get_name());
        let variant_value = LitStr::new(&variant_name, item_enum.ident.span());

        let variant: syn::Variant = syn::parse_quote! {
            #[doc = #variant_value]
            #variant_ident
        };

        variants.push(variant);
    }

    quote::quote! {
        #(#variants),*
    }
}

fn get_as_str_block(v:&Vec<Property>, item_enum: &syn::ItemEnum) -> proc_macro2::TokenStream {
    let mut arms = vec![];

    for p in v {
        let variant_name = to_rust_enum_variant_name(&p.get_name());
        let variant_ident = match make_ident(&variant_name, item_enum.ident.span()) {
            Ok(id) => id,
            Err(e) => return e.into(),
        };

        let variant_value = LitStr::new(&p.get_value(), item_enum.ident.span());

        let arm: proc_macro2::TokenStream = quote::quote! {
            Self::#variant_ident => #variant_value,
        };

        arms.push(arm);
    }

    quote::quote! {
        match self {
            #(#arms)*
        }
    }
}

fn get_from_str_block(v:&Vec<Property>, item_enum: &syn::ItemEnum) -> proc_macro2::TokenStream {
    let mut arms = vec![];

    for p in v {
        let variant_name = to_rust_enum_variant_name(&p.get_name());
        let variant_ident = match make_ident(&variant_name, item_enum.ident.span()) {
            Ok(id) => id,
            Err(e) => return e.into(),
        };

        let variant_value = LitStr::new(&p.get_value(), item_enum.ident.span());

        let arm: proc_macro2::TokenStream = quote::quote! {
            #variant_value => Some(Self::#variant_ident),
        };

        arms.push(arm);
    }

    quote::quote! {
        match s {
            #(#arms)*
            _ => None,
        }
    }
}

pub fn expand(args: InitArgs, item_enum: syn::ItemEnum) -> proc_macro2::TokenStream {
    let path = match get_file_pathbuf(args.get_path()) {
        Ok(n) => n,
        Err(e) => return e.into(),
    };

    let syn::ItemEnum {
        attrs,
        vis,
        ident,
        ..
    } = &item_enum;

    match get_ini_location_and_properties(path.display().to_string().as_str()) {
        Ok((_file_name, v)) => {
            let variants = get_enum_variants(&v, &item_enum);

            let enum_name = ident;
            let doc_str = format!("Enum variants generated from file '{}'", path.display());
            let doc = create_doc_comment(&doc_str);

            let as_str_body = get_as_str_block(&v, &item_enum);
            let from_str_body = get_from_str_block(&v, &item_enum);

            let expanded = quote::quote! {
                #doc
                #(#attrs)*
                #vis enum #enum_name {
                    #variants
                }

                impl #enum_name {
                    pub fn as_str(&self) -> &'static str {
                        #as_str_body
                    }

                    pub fn from_str(s: &str) -> Option<Self> {
                        #from_str_body
                    }
                }
            };

            expanded
        }
        Err(e) => to_error(&format!("error reading INI file: {:?}", e)).into(),
    }
}