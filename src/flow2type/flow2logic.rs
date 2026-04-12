use parser_lib::mermaid_flow::*;
use proc_macro2::TokenStream;
use std::str::FromStr;
use quote::quote;
use quote::format_ident;

use crate::init_args::*;

/// Note: this macro is designed for c language compiler, and it's not for general use.
/// Macro to generate logic code for a flowchart, with the target node as the root of the logic code. 
/// The macro takes 4 arguments: the path to the mermaid file, the name of the struct to be generated, 
/// the name of the target node in the flowchart, 
/// and the name of the field in the target node to be used as the variable in the logic code.
pub fn expand(attr: InitArgs4) -> TokenStream {
    let mmd_file_path = attr.get_path().value();
    let struct_name = attr.get_tag().to_string();
    let item_name = attr.get_tag2().to_string();
    let target = attr.get_tag3().to_string();

    let macro_name = format_ident!("ascent");
    let struct_ident = format_ident!("{}", struct_name);

    let file_content = parse_flowchart_from_path(& mmd_file_path);
    match file_content {
        Ok(mmd) => {
            
            let doc_str = format!("Struct '{}' generated from file '{}'", struct_name, mmd_file_path);
            let ascent_code = get_ascent_logic_code(&mmd, &item_name, &target).expect("Failed to generate logic code");
            let ascent_code_str : TokenStream = ascent_code.iter()
                .map(|s| TokenStream::from_str(s).unwrap())
                .collect();
            let inner = quote! {
                #[doc = #doc_str]
                pub struct #struct_ident;

                #ascent_code_str 
            };

            quote!{
                #macro_name! {
                    #inner
                }

                #[doc = #mmd_file_path]
                #[doc = #struct_name]
                fn hello() { }
            }
        },
        Err(e) => {
            let err_msg = format!("Error parsing mermaid from file '{}': {:?}", mmd_file_path, e);
            let r = quote! {
                    compile_error!(#err_msg);
                }
                .into();

            r
        }
    }
}