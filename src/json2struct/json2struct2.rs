use std::collections::HashMap;
use proc_macro2::TokenStream;
use syn::Ident;
use syn::{ItemStruct, parse_quote};

use crate::init_args::InitArgs3_2LitStr;
use parser_lib::ini::*;
use parser_lib::json::*;
use parser_lib::common::*;
use quote::quote;
use crate::utils::*;
use crate::json2struct::shared;

fn emit_type(t: &JsonType, type_mappings:&HashMap<String, String>) -> String {
    match t {
        JsonType::Scalar(s) => rust_scalar(*s).into(),
        JsonType::Array(inner) => format!("Vec<{}>", emit_type(inner, type_mappings)),
        JsonType::Optional(inner) => format!("Option<{}>", emit_type(inner, type_mappings)),
        JsonType::Object(name, _fields) => {
            if let Some(mapped) = type_mappings.get(name) {
                mapped.clone()
            } else {
                name.into()
            }
        }
    }
}

fn create_impl_block(
    struct_name: &Ident,
    field_info_list: &Vec<(Ident, syn::Type)>,
    json_type_list: &Vec<JsonType>,
    type_mappings:&HashMap<String, String>
) -> Result<TokenStream, TokenStream> {
    let resolve_object_type_name = |name: &str| {
        let type_name = pascal(name);
        type_mappings
            .get(&type_name)
            .cloned()
            .unwrap_or(type_name)
    };

    shared::create_impl_block(
        struct_name,
        field_info_list,
        json_type_list,
        &resolve_object_type_name,
    )
}

fn create_struct2(json_type:&JsonType, hash:&mut HashMap<String, TokenStream>, type_mappings:&HashMap<String, String>) -> Result<(), TokenStream> {
    match json_type {
        JsonType::Object(name, fields) => {
            let type_name = pascal(name);
            if hash.contains_key(&type_name) {
                return Ok(());
            }

            if type_mappings.contains_key(&type_name) {
                return Ok(()); // skip creating struct if type mapping exists
            }

            let struct_name_ident = Ident::new(&type_name, proc_macro2::Span::call_site());
            let mut s: ItemStruct = parse_quote! {
                struct #struct_name_ident {
                }
            };

            let mut field_info_list = vec![];
            let mut json_type_list = vec![];
            for (key, value) in fields {
                create_struct2(value, hash, type_mappings)?;

                let rust_type_str = emit_type(value, type_mappings);
                let field_name = Ident::new(key, proc_macro2::Span::call_site());
                let field_type: syn::Type = match string_to_type(&rust_type_str) {
                    Ok(t) => t,
                    Err(err) => return Err(err.into()),
                };

                field_info_list.push((field_name.clone(), field_type.clone()));
                json_type_list.push(value.clone());

                let field: syn::Field = parse_quote! {
                    pub #field_name: #field_type
                };

                if let Err(err) = push_named_field(&mut s, field) {
                    return Err(err.into());
                }
            }

            let impl_block = create_impl_block(&struct_name_ident, &field_info_list, &json_type_list, type_mappings)?;
            let token_stream = quote! {
                #s
                #impl_block
            };

            hash.insert(type_name, token_stream);
        }
        JsonType::Array(inner) => {
            create_struct2(inner, hash, type_mappings)?;
        }
        JsonType::Optional(inner) => {
            if inner.is_object() || inner.is_array() || inner.is_optional() {
                create_struct2(inner, hash, type_mappings)?;
            }
        }
        _ => { }
    }
    Ok(())
}

pub fn expand(args: InitArgs3_2LitStr) -> TokenStream {
    let struct_name_ident = args.get_tag2();

    let mut s: ItemStruct = parse_quote! {
        pub struct #struct_name_ident {
        }
    };

    let path = match get_file_pathbuf(args.get_path()) {
        Ok(n) => n,
        Err(e) => return e.into(),
    };
    
    let config_path = match get_file_pathbuf(args.get_tag()) {
        Ok(n) => n,
        Err(e) => return e.into(),
    };

    let doc_str = format!("Struct '{}' items generated from file '{}' with config '{}'", 
        struct_name_ident, path.display(), config_path.display());
    let doc = create_doc_comment(&doc_str);
    s.attrs.push(doc);

    match (parse_json_from_file(path.display().to_string().as_str()), parse_ini_from_file(config_path.display().to_string().as_str())) {
        (Ok(json_object), Ok(ini_object)) => {
            let properties = ini_object.get_property_under_section("TypeMapping");

            let type_mapping = properties.into_iter()
                                        .map(|p| p.to_tuple())
                                        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
                                        .collect::<HashMap<_, _>>();

            let mut obj_types_list : HashMap<String, TokenStream> = HashMap::new();

            let mut field_info_list = vec![];
            let mut json_type_list = vec![];

            for pair in json_object.get_pairs() {
                let (key, value) = pair.into();
                let json_type = infer_json(pascal(key), value);
                if json_type.is_object() || json_type.is_array() {
                    match create_struct2(&json_type, &mut obj_types_list, &type_mapping) {
                        Ok(_) => {},
                        Err(err) => return err,
                    }
                }
                
                let rust_type_str = if value.is_object() { pascal(& emit_type(&json_type, &type_mapping)) } 
                                else { emit_type(&json_type, &type_mapping).to_string() };
                let field_name = Ident::new(&key, proc_macro2::Span::call_site());
                let field_type: syn::Type = match string_to_type(&rust_type_str) {
                    Ok(t) => t,
                    Err(err) => return err.into(),
                };

                field_info_list.push((field_name.clone(), field_type.clone()));
                json_type_list.push(json_type);

                let field: syn::Field = parse_quote! {
                    pub #field_name: #field_type
                };

                match push_named_field(&mut s, field) {
                    Ok(_) => {}
                    Err(err) => return err,
                }
            }

            let new_types = obj_types_list.into_values();
            let impl_block = match create_impl_block(&struct_name_ident, &field_info_list, &json_type_list, &type_mapping) {
                Ok(block) => block,
                Err(err) => return err,
            };
            
            let expanded = quote! {
                #(#new_types)*

                #s
                #impl_block
            };

            expanded.into()
        }
        (Err(e), Ok(_)) => {
            let err_msg = format!("Error parsing JSON from file '{}': {:?}", path.display(), e);
            quote! {
                compile_error!(#err_msg);
            }
            .into()
        }
        (Ok(_), Err(e)) => {
            let err_msg = format!("Error parsing INI from file '{}': {:?}", config_path.display(), e);
            quote! {
                compile_error!(#err_msg);
            }
            .into()
        }
        (Err(e1), Err(e2)) => {
            let err_msg = format!("Error parsing JSON from file '{}': {:?}\nError parsing INI from file '{}': {:?}", 
                path.display(), e1, config_path.display(), e2);
            quote! {
                compile_error!(#err_msg);
            }
            .into()
        }
    }
}