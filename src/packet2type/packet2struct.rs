use parser_lib::mermaid_packet::PacketEntry;
use proc_macro::TokenStream;
use quote::quote;
use proc_macro2::Span;
use syn::{Ident, Type};

use syn::{ItemStruct, parse_macro_input};

use crate::utils::*;

pub fn bits_to_syn_type(bits: u32) -> Result<Type, TokenStream> {
    let bytes = (bits + 7) / 8;

    if bits <= 128 {
        let ty = match bits {
            1..=8 => "u8",
            9..=16 => "u16",
            17..=32 => "u32",
            33..=64 => "u64",
            _ => "u128",
        };
        syn::parse_str(ty)
            .map_err(|_| to_error(&format!("failed to map bit size {bits} to Rust type")))
    } else {
        Ok(syn::parse_quote! { [u8; #bytes] })
    }
}

/// create new function with parameters from packet entries
fn create_new(entries : &Vec<PacketEntry>) -> Result<proc_macro2::TokenStream, TokenStream> {
    let mut params = Vec::new();
    let mut inits = Vec::new();

    for entry in entries {
        let name = entry.get_name();
        let ty = bits_to_syn_type(entry.get_bit_spec().get_bit_size() as u32)?;
        let ident = Ident::new(name, Span::call_site());

        params.push(quote! {
            #ident: #ty
        });

        inits.push(quote! {
            #ident,
        });
    }

    Ok(quote! {
        pub fn new(#(#params),*) -> Self {
            Self {
                #(#inits)*
            }
        }
    })
}

/// Generate a struct from an packet file
/// /// Usage:
/// ```ignore
/// packat_struct("path/to/file.mermaid");
/// ```
pub fn expand(attr:TokenStream, input: TokenStream) -> TokenStream {
    let path = match get_file_pathbuf(&syn::parse_macro_input!(attr as syn::LitStr)) {
        Ok(n) => n,
        Err(e) => return e,
    };

    let file_path = path.display().to_string();
    let mut s = parse_macro_input!(input as ItemStruct);
    let struct_name = &s.ident;

    let doc_str = format!("Struct items generated from mermaid packet file '{}'", file_path);
    let doc = create_doc_comment(&doc_str);

    match parser_lib::mermaid_packet::parse(&file_path) {
        Ok(mut packet_section) => {
            let total_bytes = packet_section.get_total_byte_size() as usize;

            // get sorted list of entries by entry's order in packet
            packet_section.get_entries_mut()
                        .sort_by(|a, b| a.get_location().get_line().cmp(&b.get_location().get_line()));

            let entries = packet_section.get_entries();
            for entry in entries {
                let name = entry.get_name();
                let ty = match bits_to_syn_type(entry.get_bit_spec().get_bit_size() as u32) {
                    Ok(t) => t,
                    Err(e) => return e,
                };
                let ident = Ident::new(name, Span::call_site());

                let field: syn::Field = syn::parse_quote! {
                    #ident: #ty
                };

                match &mut s.fields {
                    syn::Fields::Named(fields) => {
                        fields.named.push(field);
                    }
                    _ => {
                        return syn::Error::new_spanned(
                            s,
                            "macro only supports structs with named fields"
                        )
                        .to_compile_error()
                        .into();
                    }
                }
            }

            s.attrs.push(doc);

            let serialized_size_doc = format!("Serialized size in bytes: {total_bytes} / 0x{total_bytes:X}");
            let serialized_size_attr = create_doc_comment(& serialized_size_doc);  

            let mut body = Vec::new();
            for entry in entries {
                if entry.get_byte_size() == 1 {
                    let name = entry.get_name();
                    let ident = Ident::new(name, Span::call_site());
                    body.push(quote! {
                        bytes.push(self.#ident);
                    });
                } else {
                    let name = entry.get_name();
                    let ident = Ident::new(name, Span::call_site());
                    let byte_size = entry.get_byte_size() as usize;
                    
                    if byte_size <= 16 {
                        if matches!(byte_size, 2 | 4 | 8 | 16) {
                            body.push(quote! {
                                bytes.extend_from_slice(&self.#ident.to_le_bytes());
                            });
                        }
                        else { 
                            body.push(quote! {
                                bytes.extend_from_slice(&self.#ident.to_le_bytes()[..#byte_size]);
                            });
                        }
                    }
                    else {
                        body.push(quote! {
                            bytes.extend_from_slice(&self.#ident);
                        });
                    }
                }
            }   

            let new_fn = match create_new(entries) {
                Ok(ts) => ts,
                Err(e) => return e,
            };

            let impl_block = quote! {
                impl #struct_name {
                    #serialized_size_attr
                    pub const SERIALIZED_SIZE: usize = #total_bytes; 

                    pub fn to_bytes(&self) -> Vec<u8> {
                        let s = Self::SERIALIZED_SIZE as usize;
                        let mut bytes = Vec::with_capacity(s);

                        #(#body)*

                        bytes
                    }

                    #new_fn
                }
            };

            quote!( #s 
                    #impl_block).into()
        }
        Err(e) => {
            let error_message = format!("Failed to parse packet file: {:?}", e);
            to_error(& error_message)
        }
    }
}