use std::collections::HashMap;

use parser_lib::common::*;
use parser_lib::json::*;
use syn::ItemStruct;
use proc_macro2::TokenStream;
use syn::Ident;
use syn::parse_quote;
use quote::quote;

use crate::utils::*;
use crate::json2struct::shared;

/// create impl section to host new and from_json functions
pub fn create_impl_block(
    struct_name: &Ident,
    field_info_list: &Vec<(Ident, syn::Type)>,
    json_type_list: &Vec<JsonType>
) -> Result<TokenStream, TokenStream> {
    shared::create_impl_block(
        struct_name,
        field_info_list,
        json_type_list,
        &shared::default_object_type_name,
    )
}

pub fn create_struct(json_type:&JsonType, hash:&mut HashMap<String, TokenStream>) -> Result<(), TokenStream> {
    match json_type {
        JsonType::Object(name, fields) => {
            let type_name = pascal(name);
            if hash.contains_key(&type_name) {
                return Ok(());
            }

            let struct_name_ident = Ident::new(&type_name, proc_macro2::Span::call_site());
            let mut s: ItemStruct = parse_quote! {
                struct #struct_name_ident {
                }
            };

            let mut field_info_list = vec![];
            let mut json_type_list = vec![];
            for (key, value) in fields {
                create_struct(value, hash)?;

                let rust_type_str = emit_type(value);
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

            let impl_block = create_impl_block(&struct_name_ident, &field_info_list, &json_type_list)?;
            let token_stream = quote! {
                #s
                #impl_block
            };

            hash.insert(type_name, token_stream);
        }
        JsonType::Array(inner) => {
            create_struct(inner, hash)?;
        }
        JsonType::Optional(inner) => {
            if inner.is_object() || inner.is_array() || inner.is_optional() {
                create_struct(inner, hash)?;
            }
        }
        _ => { }
    }
    Ok(())
}