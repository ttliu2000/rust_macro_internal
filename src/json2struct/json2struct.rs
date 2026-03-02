use std::collections::HashMap;

use crate::init_args::*;
use crate::json2struct::*;
use crate::utils::*;
use parser_lib::json::*;
use parser_lib::common::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;
use syn::ItemStruct;
use syn::parse_quote;

pub fn expand(args: InitArgs2) -> TokenStream {
    let struct_name_ident = args.get_tag();
    let mut s: ItemStruct = parse_quote! {
        pub struct #struct_name_ident {
        }
    };

    let path = match get_file_pathbuf(args.get_path()) {
        Ok(n) => n,
        Err(e) => return e.into(),
    };

    let doc_str = format!("Struct '{}' items generated from file '{}'", struct_name_ident, path.display());
    let doc = create_doc_comment(&doc_str);
    s.attrs.push(doc);

    match parse_json_from_file(path.display().to_string().as_str()) {
        Ok(json_object) => {
            let mut obj_types_list : HashMap<String, TokenStream> = HashMap::new();

            let mut field_info_list = vec![];
            let mut json_type_list = vec![];

            for pair in json_object.get_pairs() {
                let (key, value) = pair.into();
                let json_type = infer_json(pascal(key), value);
                if json_type.is_object() || json_type.is_array() {
                    match create_struct(&json_type, &mut obj_types_list) {
                        Ok(_) => {},
                        Err(err) => return err,
                    }
                }
                
                let rust_type_str = if value.is_object() { pascal(& emit_type(&json_type)) } 
                                else { emit_type(&json_type).to_string() };
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
            let impl_block = match create_impl_block(&struct_name_ident, &field_info_list, &json_type_list) {
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
        Err(e) => {
            let err_msg = format!("Error parsing JSON from file '{}': {:?}", path.display(), e);
            quote! {
                compile_error!(#err_msg);
            }
            .into()
        }
    }
}