use proc_macro::TokenStream;
use quote::quote;
use proc_macro2::Span;
use syn::{Field, Ident, parse_quote};

use std::env;
use std::str::FromStr;

use syn::{ItemStruct, parse_macro_input};

use crate::utils::*;

pub fn expand(attr: TokenStream, input: TokenStream) -> TokenStream {
    // get file path from attribute
    let path = match get_file_pathbuf(&syn::parse_macro_input!(attr as syn::LitStr)) {
        Ok(n) => n,
        Err(e) => return e,
    };

    let file_path = path.display().to_string();
    let mut s = parse_macro_input!(input as ItemStruct);
    let struct_name = &s.ident;

    let doc_str = format!("Struct items generated from mermaid packet file '{}'", file_path);
    let doc = create_doc_comment(&doc_str);
    s.attrs.push(doc);

    let new_field: Field = parse_quote! {
        data: Vec<u8>
    };
    if let syn::Fields::Named(fields_named) = &mut s.fields {
        fields_named.named.push(new_field);
    }

    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/data/packet_bit_vec_base.rs"));

    let tokens = match proc_macro2::TokenStream::from_str(src) {
        Ok(ts) => ts,
        Err(e) => {
            return syn::Error::new(
                Span::call_site(),
                format!("invalid Rust template in packet_bit_vec_base.rs: {e}"),
            )
            .to_compile_error()
            .into();
        }
    };
    let private_functions = quote! {
        #tokens
    };

    match parser_lib::mermaid_packet::parse(&file_path) {
        Ok(mut packet_section) => {
            let total_bytes = packet_section.get_total_byte_size() as usize;

            // get sorted list of entries by entry's order in packet
            packet_section.get_entries_mut()
                        .sort_by(|a, b| a.get_location().get_line().cmp(&b.get_location().get_line()));

            let entries = packet_section.get_entries();

            let mut offset = 0;
            let mut functions = Vec::new();
            for entry in entries {
                let name = entry.get_name();
                let size = entry.get_bit_spec().get_bit_size() as usize;
                let function_name = Ident::new(&format!("set_{}_bits", name), Span::call_site());
                let f = quote! {
                    /// Set bits for field: #name
                    pub fn #function_name(&mut self, value: u64) {
                        self.set_bit_range_value(#offset, #offset + #size, value);
                    }
                };
                functions.push(f);
                offset += size;
            }

            let impl_part = quote! {
                            impl #struct_name {
                                /// Create a new instance with allocated data vector
                                pub fn new() -> Self {
                                    Self {
                                        data: vec![0u8; #total_bytes],
                                    }
                                }

                                #(#functions)*

                                #private_functions
                            }
                        };

            quote! {
                #s
                #impl_part
            }.into()
        }
        Err(e) => {
            return syn::Error::new(Span::call_site(),
                format!("Failed to parse mermaid packet file '{}': {:?}", file_path, e))
                .to_compile_error()
                .into();
        }
    }
    
}