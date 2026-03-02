use std::collections::HashMap;

use parser_lib::mermaid_sequence::*;
use syn::{ItemFn, LitStr};

use crate::init_args::InitArgs;
use crate::utils::*;

fn get_parameter_name(str: &str) -> String {
    to_rust_var_name(str, "")
}

fn get_parameter_core_type(str: &str) -> String {
    to_rust_type_name(str, "I")
}

fn get_parameters(name_aliases:&Vec<(String, String)>) -> proc_macro2::TokenStream {
    let mut params = vec![];
    for (name, _) in name_aliases {
        let param_ident = syn::Ident::new(&get_parameter_name(name), proc_macro2::Span::call_site());
        let param_type = syn::Ident::new(&get_parameter_core_type(name), proc_macro2::Span::call_site());
        params.push(quote::quote! {
            #param_ident: &mut dyn #param_type
        });
    }

    quote::quote! {
        #(#params),*
    }
}

fn get_trait_methods(name:&str, alias:&str, seq:&SequenceProgram) -> proc_macro2::TokenStream {
    let mut methods = vec![];
    let messages = seq.get_messages_by_name_alias(name, alias);

    // generate trait methods defintion for reach message
    for msg in messages {
        let method_name = syn::Ident::new(&to_rust_fn_name(&msg.get_message_text()), proc_macro2::Span::call_site());
        let partner_type = syn::Ident::new(&get_parameter_core_type(msg.get_message_actor_to()), proc_macro2::Span::call_site());
        let msg = LitStr::new(msg.get_message_text(), proc_macro2::Span::call_site());
        methods.push(quote::quote! {
            fn #method_name(&mut self, partner:&mut dyn #partner_type) { println!("{}", #msg) }
        });
    }

    quote::quote! {
        #(#methods)*
    }
}

fn create_trait_types(name_aliases:&Vec<(String, String)>, seq:&SequenceProgram) -> proc_macro2::TokenStream {
    let mut types = vec![];
    for (name, _) in name_aliases {
        let type_ident = syn::Ident::new(&get_parameter_core_type(name), proc_macro2::Span::call_site());
        let funcs = get_trait_methods(name, name, seq);
        types.push(quote::quote! {
            pub trait #type_ident {
                // define trait methods here if needed
                #funcs
            }
        });
    }

    quote::quote! {
        #(#types)*
    }
}

fn get_function_invoke_list(name_aliases:&Vec<(String, String)>, seq:&SequenceProgram) -> proc_macro2::TokenStream {

    fn alias_to_name_map(name_aliases: &Vec<(String, String)>) -> HashMap<String, String> {
        name_aliases
            .iter()
            .map(|(name, alias)| (alias.clone(), name.clone()))
            .collect()
    }
    let alias_name_map = alias_to_name_map(name_aliases);

    let mut invokes = vec![];

    for stmt in &seq.stmts {
        if let Some(msg) = stmt.get_message_statement() {
            let from_name = alias_name_map.get(msg.get_message_actor_from())
                            .unwrap_or(msg.get_message_actor_from());
            let to_name = alias_name_map.get(msg.get_message_actor_to())
                            .unwrap_or(msg.get_message_actor_to());

            let from_param = syn::Ident::new(&get_parameter_name(from_name), proc_macro2::Span::call_site());
            let method_name = syn::Ident::new(&to_rust_fn_name(&msg.get_message_text()), proc_macro2::Span::call_site());
            let to_param = syn::Ident::new(&get_parameter_name(to_name), proc_macro2::Span::call_site());

            invokes.push(quote::quote! {
                #from_param.#method_name(#to_param);
            });
        }
    }
    

    quote::quote! {
        #(#invokes);*
    }
}

pub fn expand(args:InitArgs, item:ItemFn) -> proc_macro2::TokenStream {
    let path = args.get_path();
    let func_name = &item.sig.ident;

    // user cannot define the parameters
    if !item.sig.inputs.is_empty() {
        return to_error("do not define parameters; the macro defines the signature").into();
    }

    match parse_sequence_from_file(&path.value()) {
        Err(e) => {
            return to_error(&format!("error parsing sequence diagram file: {:?}", e)).into();
        }
        Ok(seq) => {
            let name_aliases = seq.get_participant_name_alias();
            let parameters = get_parameters(&name_aliases);
            let trait_types = create_trait_types(&name_aliases, &seq);

            let function_invokes = get_function_invoke_list(&name_aliases, &seq);

            let expanded = quote::quote! {
                pub fn #func_name(#parameters) {
                    #function_invokes
                }

                #trait_types
            };

            expanded
        }
    }
}