use std::collections::HashSet;

use parser_lib::csv::*;
use proc_macro2::TokenStream;
use syn::Ident;
use syn::LitStr;

use crate::init_args::*;
use crate::utils::*;
use quote::quote;

fn equalize(a: &mut Vec<String>, b: &mut Vec<String>) {
    let max = a.len().max(b.len());

    a.resize_with(max, String::new);
    b.resize_with(max, String::new);
}

fn unique_in_place<'a>(v: &mut Vec<&'a str>) {
    let mut seen = HashSet::new();
    v.retain(|s| seen.insert(*s));
}

fn create_map_inserter(keys: &Vec<&str>, values: &Vec<&str>, key_converter_function:&str, value_converter_function:&str) -> TokenStream {
    let key_converter_function = Ident::new(key_converter_function, proc_macro2::Span::call_site());
    let value_converter_function = Ident::new(value_converter_function, proc_macro2::Span::call_site());
    
    let null_in_key = keys.iter().any(|k| k.is_empty());
    let null_in_value = values.iter().any(|v| v.is_empty());

    let inserts: Vec<_> = keys.iter().zip(values.iter()).map(|(k, v)| {
        let key_litstr = LitStr::new(*k, proc_macro2::Span::call_site());
        let value_litstr = LitStr::new(*v, proc_macro2::Span::call_site());
        if null_in_key || null_in_value {
            quote! {
                map.insert(#key_converter_function(#key_litstr), #value_converter_function(#value_litstr));
            }
        }
        else {
            quote! {
                map.insert(#key_converter_function(#key_litstr).unwrap(), #value_converter_function(#value_litstr).unwrap());
            }
        }
        
    }).collect();

    quote! {
        #(#inserts)*
    }
}

pub fn expand(args : InitArgs3) -> TokenStream {
    let path = match get_file_pathbuf(args.get_path()) {
        Ok(n) => n,
        Err(e) => return e.into(),
    };

    match parse_csv_file(path.display().to_string().as_str()) {
        Ok(csv_file) => {
            let key_col_name = args.get_tag().to_string();
            let value_col_name = args.get_tag2().to_string();

            let key_col = csv_file.get_column_by_name(&key_col_name);
            let value_col = csv_file.get_column_by_name(&value_col_name);

            match (key_col, value_col) {
                (Some(mut keys), Some(mut values)) => {
                    unique_in_place(&mut keys);
                    unique_in_place(&mut values);
                    equalize(&mut keys.iter_mut().map(|s| s.to_string()).collect(), &mut values.iter_mut().map(|s| s.to_string()).collect());
                    let keys_str: Vec<&str> = keys.iter().map(|s| *s).collect();
                    let values_str: Vec<&str> = values.iter().map(|s| *s).collect();
                    let key_type = infer_column(keys_str);
                    let value_type = infer_column(values_str);

                    let key_converter_func = match convert_function(&key_type.0) {
                        Some(f) => f,
                        None => {
                            let err_msg = format!(
                                "No converter found for inferred key type '{:?}'",
                                key_type.0
                            );
                            return to_error(&err_msg).into();
                        }
                    };
                    let value_converter_func = match convert_function(&value_type.0) {
                        Some(f) => f,
                        None => {
                            let err_msg = format!(
                                "No converter found for inferred value type '{:?}'",
                                value_type.0
                            );
                            return to_error(&err_msg).into();
                        }
                    };

                    let inserter = create_map_inserter(&keys, &values, &key_converter_func, &value_converter_func);

                    quote! {
                        {
                            let mut map = ::std::collections::HashMap::new();
                            #inserter
                            map
                        }
                    }
                }
                _ => {
                    let err_msg = format!(
                        "Failed to find columns '{}' or '{}' in CSV file '{}'",
                        key_col_name,
                        value_col_name,
                        path.display()
                    );
                    
                    return to_error(&err_msg).into();
                }
            }
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