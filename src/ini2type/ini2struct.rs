use proc_macro::TokenStream;
use crate::utils::*;
use parser_lib::ini::*;

use syn::{ItemStruct, LitStr, parse_macro_input};
use syn::{
    parse::{Parse, ParseStream},
    Result,
};

struct IniStructArgs {
    path: LitStr,
}

impl Parse for IniStructArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let path: LitStr = input.parse()?;
        Ok(Self { path })
    }
}

/// Generate a struct from an INI file
/// /// Usage:
/// ```ignore
/// ini_struct("path/to/file.ini");
/// ```
/// The INI file should contain key-value pairs where keys are used as struct field names
/// and values are used as their corresponding types.
pub fn expand(attr: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as IniStructArgs);
    let path = match get_file_pathbuf(&args.path) {
        Ok(n) => n,
        Err(e) => return e,
    };

    let mut s = parse_macro_input!(input as ItemStruct);
    let struct_name = &s.ident;

    let doc_str = format!("Struct items generated from file '{}'", path.display());
    let doc = create_doc_comment(&doc_str);
    s.attrs.push(doc);

    match get_ini_location_and_properties(&path.display().to_string()) {
        Ok((_file_name, v)) => {
            for p in v.iter() {
                let ident = match make_ident(&p.get_name(), struct_name.span()) {
                    Ok(id) => id,
                    Err(e) => return e,
                };

                let ty = match parse_type_or_error(p.get_value()) {
                    Ok(t) => t,
                    Err(e) => return e,
                };
                
                let field: syn::Field = syn::parse_quote! {
                    #ident: #ty
                };

                match &mut s.fields {
                    syn::Fields::Named(fields) => {
                        fields.named.push(field);
                    }
                    _ => {
                        let err = syn::Error::new_spanned(
                            s,
                            "this macro only supports structs with named fields"
                        )
                        .to_compile_error();
                        return err.into();
                    }
                }
            }

            quote::quote!(#s).into()
        }
        Err(e) => {
            let file_path = path.display().to_string();
            let err_msg = format!("failed to parse/process INI file '{}': {:?}", file_path, e);
            to_error(&err_msg)
        }
    }
}