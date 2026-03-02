use std::collections::HashSet;

use parser_lib::mermaid_state::*;
use proc_macro2::TokenStream;
use syn::{Ident, ItemStruct};
use quote::{format_ident, quote};
use super::*;

use crate::{init_args::*, utils::*};

fn create_state(item:&ItemStruct, state_graph:&StateGraphProgram) -> TokenStream {
    let mut states = vec![];
    for stmt in state_graph.stmts.iter() {
        if let Some(seq_stmt) = stmt.get_state_sequence_statement() {
            states.extend(seq_stmt.get_state_names().into_iter().map(|x| to_rust_enum_variant_name(&x)));
        }
    }
    states.sort();
    states.dedup();

    let state_idents = states.into_iter().map(|x| {
        Ident::new(&x, proc_macro2::Span::call_site())
     }).collect::<Vec<_>>();

    let enum_name_ident = get_state_type_name(&item);
    quote! {
        pub enum #enum_name_ident {
            #(#state_idents),*
        }
    }
}

fn create_trigger_event(item:&ItemStruct, state_graph:&StateGraphProgram) -> TokenStream {
    let struct_name = &item.ident;
    let trigger_events = state_graph
        .stmts
        .iter()
        .filter_map(|x| x.get_state_sequence_statement())
        .map(|stmt| to_rust_enum_variant_name(&stmt.get_description()))
        .collect::<Vec<_>>();

    let has_dup = trigger_events.len() != trigger_events.iter().collect::<HashSet<_>>().len();
    if has_dup {
        let err_msg = format!("Duplicate trigger event names in state diagram for struct {}, please check ", struct_name);
        return quote! {
            compile_error!(#err_msg);
        };
    }

    let trigger_event_idents = trigger_events.into_iter().map(|x| {
        format_ident!("{}", x)
     }).collect::<Vec<_>>();

    let enum_name = get_trigger_event_type_name(item);
    quote! {
        pub enum #enum_name {
            #(#trigger_event_idents),*
        }
    }
}

fn get_state_transition_match_arms(state_graph:&StateGraphProgram, item:&ItemStruct) -> Vec<TokenStream> {
    let state_name_type = get_state_type_name(&item);
    let event_name_type = get_trigger_event_type_name(&item);
    state_graph.stmts.iter().filter_map(|x| x.get_state_sequence_statement()).map(|stmt| {
        let start_state = format_ident!("{}", to_rust_enum_variant_name(&stmt.get_start_state_name()));
        let end_state = format_ident!("{}", to_rust_enum_variant_name(&stmt.get_end_state_name()));
        let event_name = format_ident!("{}", to_rust_enum_variant_name(&stmt.get_description()));
        quote! {
            (#state_name_type::#start_state, #event_name_type::#event_name) => {
                    self.current_state = #state_name_type::#end_state;
                }
        }
    }).collect::<Vec<_>>()
}

fn get_state_transition_peek_match_arms(state_graph:&StateGraphProgram, item:&ItemStruct) -> Vec<TokenStream> {
    let state_name_type = get_state_type_name(&item);
    let event_name_type = get_trigger_event_type_name(&item);
    state_graph.stmts.iter().filter_map(|x| x.get_state_sequence_statement()).map(|stmt| {
        let start_state = format_ident!("{}", to_rust_enum_variant_name(&stmt.get_start_state_name()));
        let end_state = format_ident!("{}", to_rust_enum_variant_name(&stmt.get_end_state_name()));
        let event_name = format_ident!("{}", to_rust_enum_variant_name(&stmt.get_description()));
        quote! {
            (#state_name_type::#start_state, #event_name_type::#event_name) => {
                    Some(#state_name_type::#end_state)
                }
        }
    }).collect::<Vec<_>>()
}

fn create_struct_impl(item:&ItemStruct, state_graph:&StateGraphProgram) -> TokenStream {
    let struct_name = &item.ident;
    let event_type_name = get_trigger_event_type_name(&item);
    let match_statements = get_state_transition_match_arms(state_graph, item);
    let state_type_name = get_state_type_name(&item);
    let match_statements_peek = get_state_transition_peek_match_arms(state_graph, item);
    quote! {
        impl #struct_name {
            // Additional methods can be added here
            pub fn transit(&mut self, event: & #event_type_name) {
                match (& self.current_state, event) {
                    #(#match_statements)*
                    _ => {

                    }
                }
            }

            pub fn peek_next_state(&self, event: & #event_type_name) -> Option<#state_type_name> {
                match (& self.current_state, event) {
                    #(#match_statements_peek)*
                    _ => {
                        None
                    }
                }
            }
        }
    }
}

pub fn expand(attr: InitArgs, mut item: ItemStruct) -> TokenStream {
    let file_path = attr.get_path();

    let doc = format!("State struct generated from file: {}", file_path.value());
    let attr = create_doc_comment(&doc);
    item.attrs.push(attr);

    // add fields to struct for state and trigger event enums
    let state_enum_name = get_state_type_name(&item);
    match push_named_field(&mut item, syn::parse_quote! {
        current_state: #state_enum_name
    }) {
        Ok(_) => {}
        Err(e) => {
            return e;
        }
    }

    match parse_state_from_file(&file_path.value()) {
        Ok(state_graph) => {
            if !state_graph.stmts.iter().all(|x| x.is_statesequencestatement()) {
                let err_msg = format!("Only state sequence diagrams in state_struct macro: {}", file_path.value());
                return quote! {
                    compile_error!(#err_msg);
                };
            }

            let state_enum = create_state(&item, &state_graph);
            let trigger_enum = create_trigger_event(&item, &state_graph);
            let impl_block = create_struct_impl(&item, &state_graph);

            quote! {
                #item
                #impl_block

                #state_enum

                #trigger_enum
            }
        }
        Err(e) => {
            let err_msg = format!("Error parsing state diagram from file {}: {:?}", file_path.value(), e);
            return quote! {
                compile_error!(#err_msg);
            };
        }
    }
}