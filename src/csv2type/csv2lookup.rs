use proc_macro2::TokenStream;
use syn::Ident;
use syn::Type;

use crate::init_args::*;
use crate::utils::*;
use parser_lib::csv::*;
use syn::{Lit, parse_str};

/// create function name like **lookup_size_enumname**, and the function will return the value based on the enum variant, 
/// if the value is empty in csv, it will return None, otherwise it will return Some(value)
fn create_lookup_function(names:&Vec<&str>, value:&Vec<&str>, ty:&Type, enum_ident:&Ident) -> TokenStream {
    let mut entries = Vec::new();

    for (name, val) in names.iter().zip(value.iter()) {
        let key_name = syn::Ident::new(*name, proc_macro2::Span::call_site());
        if val.is_empty() {
            let r = quote::quote! {
                #enum_ident::#key_name => None,
            };
            entries.push(r);
        }
        else {
            let value_name : Lit = match parse_str(*val) {
                Ok(v) => v,
                Err(_) => {
                    return to_error(&format!("invalid literal '{}' in lookup table", val)).into();
                }
            };
            let r = quote::quote! {
                #enum_ident::#key_name => Some(#value_name),
            };
            entries.push(r);
        }
    }

    let return_type = ty;

    let function_name_str = format!("lookup_size_{}", enum_ident.to_string().to_lowercase());
    let function_name = syn::Ident::new(&function_name_str, proc_macro2::Span::call_site());

    quote::quote! {
        impl #enum_ident {
            fn #function_name(&self) -> #return_type {
                match self {
                    #(#entries)*
                    _ => None,
                }
            }
        }
    }
}

/// create function name like **get_enumname_name**, and the function will return the name string based on the enum variant
fn create_get_name_function(names:&Vec<&str>, enum_ident:&Ident) -> TokenStream {
    let mut entries = Vec::new();

    for name in names.iter() {
        let key_name = syn::Ident::new(*name, proc_macro2::Span::call_site());
        let value_name = syn::LitStr::new(*name, proc_macro2::Span::call_site());
        let r = quote::quote! {
            #enum_ident::#key_name => #value_name,
        };
        entries.push(r);
    }

    let function_name_str = format!("get_{}_name", enum_ident.to_string().to_lowercase());
    let function_name = syn::Ident::new(&function_name_str, proc_macro2::Span::call_site());

    quote::quote! {
        impl #enum_ident {
            fn #function_name(&self) -> &'static str {
                match self {
                    #(#entries)*
                    _ => "Invalid",
                }
            }
        }
    }
}

pub fn expand(args : InitArgs4) -> TokenStream {
    let path = match get_file_pathbuf(args.get_path()) {
        Ok(n) => n,
        Err(e) => return e.into(),
    };

     match parse_csv_file(path.display().to_string().as_str()) {
        Ok(csv_file) => {
            let key_col_name = args.get_tag().to_string();
            let value_col_name = args.get_tag2().to_string();

            let key_index = match csv_file.find_column_index_by_name(&key_col_name) {
                Some(n) => n,
                None => {
                    let err_msg = format!(
                        "Key column '{}' not found in CSV file '{}'",
                        key_col_name,
                        path.display()
                    );
                    return to_error(&err_msg).into();
                }
            };

            let value_index = match csv_file.find_column_index_by_name(&value_col_name) {
                Some(n) => n,
                None => {
                    let err_msg = format!(
                        "Value column '{}' not found in CSV file '{}'",
                        value_col_name,
                        path.display()
                    );
                    return to_error(&err_msg).into();
                }
            };

            let key_data = csv_file.get_column_data(key_index);
            let value_data = csv_file.get_column_data(value_index);

            let enum_name = args.get_tag3().to_string();
            let enum_ident = syn::Ident::new(&enum_name, proc_macro2::Span::call_site());

            let return_type = infer_column(value_data.clone());
            let return_type = match string_to_type(&rust_type(return_type.0, return_type.1)) {
                Ok(t) => {
                    if is_option(&t) {
                        t
                    } else {
                        wrap_type_in_option(&t)
                    }
                }
                Err(err) => return err.into(),
            };

            let lookup_function = create_lookup_function(
                &key_data,
                &value_data,
                &return_type,
                &enum_ident,
            );

            let get_name_function = create_get_name_function(
                &key_data,
                &enum_ident,
            );

            quote::quote! {
                #lookup_function

                #get_name_function
            }.into()
        }
        Err(e) => {
            let err_msg = format!(
                "Failed to parse CSV file '{}': {:?}",
                path.display(),
                e
            );
            
            to_error(&err_msg).into()
        }
    }
}