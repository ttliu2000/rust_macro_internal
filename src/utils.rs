use std::collections::HashSet;
use std::path::PathBuf;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{Attribute, GenericArgument, Ident, ItemStruct, LitInt, LitStr, PathArguments, TypeReference, TypeTuple};
use syn::{Field, Fields};
use syn::TypePath;
use syn::Type;
use quote::quote;

pub fn get_file_pathbuf(path_lit: &LitStr) -> Result<PathBuf, TokenStream> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        TokenStream::from(
            syn::Error::new_spanned(path_lit, "CARGO_MANIFEST_DIR is not set").to_compile_error(),
        )
    })?;

    let rel_path = path_lit.value();
    let mut path = PathBuf::from(manifest_dir.clone());
    path.push(&rel_path);
    
    // check if file exists
    if path.exists() {
        Ok(path)
    }
    else {
        let mut fallback = PathBuf::from(manifest_dir.clone());
        fallback.push("riscv_asm_lib");
        fallback.push(&rel_path);

        if fallback.exists() {
            return Ok(fallback);
        }

        let root_folder_info = format!("the manifest dir = {manifest_dir}, path = {}, and fallbak path to file = {}", 
            path.display(), 
            fallback.display());
        let err_msg = format!("The specified file is not exists. {root_folder_info}");
        let token = syn::Error::new_spanned(
            path_lit,
            err_msg,
            )
            .to_compile_error()
            .into();

        Err(token)
    }
}

pub fn create_doc_comment(comment: &str) -> Attribute {
    let doc_text = LitStr::new(comment, Span::call_site());
    syn::parse_quote! {
        #[doc = #doc_text]
    }
}

pub fn to_error(s: &str) -> TokenStream {
    syn::Error::new(Span::call_site(), s)
        .to_compile_error()
        .into()
}

pub fn make_ident(s: &str, span: Span) -> Result<Ident, TokenStream> {
    if syn::parse_str::<Ident>(s).is_ok() {
        Ok(Ident::new(s, span))
    } else {
        let err = syn::Error::new(span, format!("`{}` is not a valid Rust identifier", s));
        let token_stream = err.to_compile_error().into();
        Err(token_stream)
    }
}

/// Parse a string into a `syn::Type`, or emit a compile-time error.
pub fn parse_type_or_error(repr_string: &str) -> Result<Type, TokenStream> {
    match syn::parse_str::<Type>(repr_string) {
        Ok(ty) => Ok(ty),
        Err(_) => Err(
            syn::Error::new(
                Span::call_site(),
                format!("`{}` is not a valid Rust type", repr_string),
            )
            .to_compile_error()
            .into(),
        ),
    }
}

/// Convert a string representation of a type into a `syn::Type`.
pub fn string_to_type(ty: &str) -> Result<Type, proc_macro2::TokenStream> {
    syn::parse_str::<Type>(ty).map_err(|e| {
        let msg = e.to_string();
        quote! {
            compile_error!(#msg);
        }
    })
}

pub fn push_named_field(
    s: &mut ItemStruct,
    field: Field,
) -> Result<(), proc_macro2::TokenStream> {
    match &mut s.fields {
        Fields::Named(fields_named) => {
            fields_named.named.push(field);
            Ok(())
        }
        Fields::Unnamed(_) => Err(quote! {
            compile_error!("expected a struct with named fields, found tuple struct");
        }),
        Fields::Unit => Err(quote! {
            compile_error!("expected a struct with named fields, found unit struct");
        }),
    }
}

pub fn create_new_function(fn_name:&Ident, fields:&Vec<(Ident, Type)>) -> proc_macro2::TokenStream {
    let args: Vec<_> = fields.iter().map(|(ident, ty)| {
        quote! { #ident: #ty }
    }).collect();

    let inits: Vec<_> = fields.iter().map(|(ident, _)| {
        quote! { #ident }
    }).collect();

    quote! {
        pub fn #fn_name( #(#args),* ) -> Self {
            Self {
                #(#inits),*
            }
        }
    }
}

pub fn is_option(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            type_path.qself.is_none() &&
            type_path.path.segments.last().is_some_and(|seg| {
                seg.ident == "Option" &&
                matches!(seg.arguments, PathArguments::AngleBracketed(_))
            })
        }
        _ => false,
    }
}

pub fn wrap_type_in_option(ty: &Type) -> Type {
    syn::parse_quote! {
        Option<#ty>
    }
}

/// create type ref from type, for example, for type T, it will create &T
pub fn create_type_ref(ty: &Type) -> Type {
    let ref_inner = Type::Reference(TypeReference {
        and_token: Default::default(),
        lifetime: None,
        mutability: None,
        elem: Box::new(ty.clone()),
    });

    ref_inner
}

pub fn option_t_to_option_ref_t(ty: &Type) -> Option<Type> {
    if !is_option(ty) {
        return None;
    }
    
    let Type::Path(tp) = ty else { return None };

    let last = tp.path.segments.last()?;
    if last.ident != "Option" {
        return None;
    }

    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return None;
    };

    let GenericArgument::Type(inner_ty) = args.args.first()? else {
        return None;
    };

    // Build &T
    let ref_inner = create_type_ref(inner_ty);

    // Rebuild Option<&T>
    let mut new_tp = tp.clone();
    let last_mut = new_tp.path.segments.last_mut()?;

    last_mut.arguments = PathArguments::AngleBracketed(
        syn::AngleBracketedGenericArguments {
            colon2_token: None,
            lt_token: Default::default(),
            args: {
                let mut p = Punctuated::<GenericArgument, Comma>::new();
                p.push(GenericArgument::Type(ref_inner));
                p
            },
            gt_token: Default::default(),
        }
    );

    Some(Type::Path(new_tp))
}

/// Check if a type is a primitive scalar type (e.g., i32, f64, bool, char).
pub fn is_primitive_scalar(ty: &Type) -> bool {
    match ty {
        Type::Path(TypePath { qself: None, path }) => {
            if path.segments.len() != 1 {
                return false;
            }

            let ident = &path.segments[0].ident;
            matches!(
                ident.to_string().as_str(),
                "i8" | "i16" | "i32" | "i64" | "i128" |
                "u8" | "u16" | "u32" | "u64" | "u128" |
                "isize" | "usize" |
                "f32" | "f64" |
                "bool" | "char"
            )
        }
        _ => false,
    }
}

fn convert_type(ty: &Type) -> (Option<Type>, Option<proc_macro2::TokenStream>) {
    if is_option(ty) {
        (option_t_to_option_ref_t(ty), Some(quote! { as_ref() }))
    } else {
        (None, None)
    }
}

pub fn create_getters(fields:&Vec<(Ident, Type)>) -> proc_macro2::TokenStream {
    let getters: Vec<_> = fields.iter().map(|(ident, ty)| {
        let getter_name = Ident::new(&format!("get_{}", ident), Span::call_site());
        let is_primitive = is_primitive_scalar(ty);
        match convert_type(ty) {
            (Some(ty0), Some(postfix)) => {
                quote! {
                    pub fn #getter_name(&self) -> #ty0 {
                        self.#ident.#postfix
                    }
                }
            }
            (Some(ty0), None) => {
                quote! {
                    pub fn #getter_name(&self) -> #ty0 {
                        &self.#ident
                    }
                }
            }
            (None, None) => {
                if is_primitive {
                    quote! {
                        pub fn #getter_name(&self) -> #ty {
                            self.#ident
                        }
                    }
                } else {
                     quote! {
                        pub fn #getter_name(&self) -> &#ty {
                            &self.#ident
                        }
                    }
                }
            }
            _ => {
                quote! {
                    compile_error!("unexpected conversion result from convert_type");
                }
            }
        }
    }).collect();

    quote! {
        #(#getters)*
    }
}

pub fn create_getters_with_names(fields:&Vec<(Ident, Type)>, column_names:&Vec<&str>) -> proc_macro2::TokenStream {
    let getters: Vec<_> = fields.iter().enumerate().map(|(index, (ident, ty))| {
        let column_name = column_names.get(index).unwrap_or(&"");
        let getter_name = Ident::new(&format!("get_{}", column_name), Span::call_site());
        let is_primitive = is_primitive_scalar(ty);
        match convert_type(ty) {
            (Some(ty0), Some(postfix)) => {
                quote! {
                    /// Getter for column: #column_name
                    pub fn #getter_name(&self) -> #ty0 {
                        self.#ident.#postfix
                    }
                }
            }
            (Some(ty0), None) => {
                quote! {
                    /// Getter for column: #column_name
                    pub fn #getter_name(&self) -> #ty0 {
                        &self.#ident
                    }
                }
            }
            (None, None) => {
                if is_primitive {
                    quote! {
                        /// Getter for column: #column_name
                        pub fn #getter_name(&self) -> #ty {
                            self.#ident
                        }
                    }
                } else {
                     quote! {
                        /// Getter for column: #column_name
                        pub fn #getter_name(&self) -> &#ty {
                            &self.#ident
                        }
                    }
                }
            }
            _ => {
                quote! {
                    compile_error!("unexpected conversion result from convert_type");
                }
            }
        }
    }).collect();

    quote! {
        #(#getters)*
    }
}

/// create function to create tuple type.
/// **Note**: this function does not process single element case
pub fn create_tuple_type(types: Vec<&Type>) -> Result<Type, TokenStream> {
    if types.len() <= 1 {
        return Err(to_error("tuple type must have at least 2 elements"));
    }

    let mut elems: Punctuated<Type, Comma> = Punctuated::new();

    for ty in types {
        elems.push(ty.clone());
    }

    Ok(Type::Tuple(TypeTuple {
        paren_token: Default::default(),
        elems,
    }))
}

pub fn create_setters(fields:&Vec<(Ident, Type)>) -> proc_macro2::TokenStream {
    let setters: Vec<_> = fields.iter().map(|(ident, ty)| {
        let setter_name = Ident::new(&format!("set_{}", ident), Span::call_site());
        quote! {
            pub fn #setter_name(&mut self, value: #ty) {
                self.#ident = value;
            }
        }
    }).collect();

    quote! {
        #(#setters)*
    }
}

pub fn create_setters_with_names(fields:&Vec<(Ident, Type)>, column_names:&Vec<&str>) -> proc_macro2::TokenStream {
    let setters: Vec<_> = fields.iter().enumerate().map(|(index, (ident, ty))| {
        let column_name = column_names.get(index).unwrap_or(&"");
        let setter_name = Ident::new(&format!("set_{}", column_name), Span::call_site());
        quote! {
            /// Setter for column: #column_name
            pub fn #setter_name(&mut self, value: #ty) {
                self.#ident = value;
            }
        }
    }).collect();

    quote! {
        #(#setters)*
    }
}

pub fn create_litint(n:&str) -> Result<LitInt, TokenStream> {
    match n.parse::<u64>() {
        Ok(_) => Ok(LitInt::new(n, Span::call_site())),
        Err(_) => {
            let err = syn::Error::new(Span::call_site(), format!("`{}` is not a valid integer literal", n));
            let token_stream = err.to_compile_error().into();
            Err(token_stream)
        }
    }
}

/// convert a string to rust variable name
pub fn to_rust_var_name(s: &str, prefix:&str) -> String {
    let mut result = String::new();
    let mut prev_was_underscore = false;

    for c in s.chars() {
        if c.is_alphanumeric() {
            result.push(c.to_ascii_lowercase());
            prev_was_underscore = false;
        } else {
            if !prev_was_underscore {
                result.push('_');
                prev_was_underscore = true;
            }
        }
    }

    // Remove trailing underscore if exists
    if result.ends_with('_') {
        result.pop();
    }

    // Ensure it doesn't start with a digit
    if let Some(first_char) = result.chars().next() {
        if first_char.is_digit(10) {
            result = format!("{prefix}_{}", result);
        }
    }

    result
}

/// convert string to enum variant name
pub fn to_rust_enum_variant_name(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c.is_alphanumeric() {
            if capitalize_next {
                result.push(c.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                result.push(c.to_ascii_lowercase());
            }
        } else {
            capitalize_next = true;
        }
    }

    result
}

/// convert string to type name
pub fn to_rust_type_name(s: &str, prefix:&str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c.is_alphanumeric() {
            if capitalize_next {
                result.push(c.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                result.push(c.to_ascii_lowercase());
            }
        } else {
            capitalize_next = true;
        }
    }

    result = format!("{}{}", prefix, result);

    result
}

/// convert string to rust function name
pub fn to_rust_fn_name(input: &str) -> String {
    // Rust keywords (not exhaustive, but covers common ones)
    let keywords: HashSet<&'static str> = [
        "as", "break", "const", "continue", "crate", "else", "enum",
        "extern", "false", "fn", "for", "if", "impl", "in", "let",
        "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super",
        "trait", "true", "type", "unsafe", "use", "where", "while",
        "async", "await", "dyn",
    ]
    .into_iter()
    .collect();

    let mut out = String::new();
    let mut prev_underscore = false;

    for (i, ch) in input.chars().enumerate() {
        if ch.is_ascii_alphanumeric() {
            // Rust identifiers cannot start with a digit
            if i == 0 && ch.is_ascii_digit() {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            prev_underscore = false;
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
    }

    // Trim trailing underscores
    while out.ends_with('_') {
        out.pop();
    }

    // Empty fallback
    if out.is_empty() {
        out.push_str("func");
    }

    // Avoid keywords
    if keywords.contains(out.as_str()) {
        out.push('_');
    }

    out
}

pub fn ident_from_str(s: &str) -> Ident {
    Ident::new(s, proc_macro2::Span::call_site())
}
