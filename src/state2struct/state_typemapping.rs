use proc_macro2::TokenStream;

use crate::init_args::InitArgs2;
use crate::utils::*;
use parser_lib::mermaid_state::*;

fn generate_from(from:&str, to:&str, function_name:&str)->Result<TokenStream, TokenStream>{
    let from_ident : TokenStream = from
        .parse()
        .map_err(|_| proc_macro2::TokenStream::from(to_error(&format!("invalid source type in state mapping: {from}"))))?;
    let to_ident : TokenStream = to
        .parse()
        .map_err(|_| proc_macro2::TokenStream::from(to_error(&format!("invalid target type in state mapping: {to}"))))?;
    let function_ident = ident_from_str(function_name);
 
    Ok(quote::quote!{
        impl From<#from_ident> for #to_ident {
            fn from(state: #from_ident) -> Self {
                #to_ident::#function_ident(state)
            }
        }
    })
}

pub fn expand_type_mapping(args: InitArgs2) -> TokenStream {
    let path = match get_file_pathbuf(args.get_path()) {
        Ok(n) => n,
        Err(e) => return e.into(),
    };

    match parse_state_from_file(path.display().to_string().as_str()) {
        Ok(state_graph) => {
            let init_type_name = args.get_tag().to_string();
            if init_type_name.to_uppercase() == "ALL" {
                let triples = state_graph.get_all_type_conversion_triples();
                let from_code_list = match triples
                    .iter()
                    .map(|(from_type, to_type, conversion_function)| {
                        generate_from(
                            from_type,
                            to_type,
                            conversion_function,
                        )
                    })
                    .collect::<Result<Vec<TokenStream>, TokenStream>>() {
                        Ok(v) => v,
                        Err(e) => return e,
                    };

                quote::quote! {
                    #(#from_code_list)* 
                }.into()
            }
            else {
                let neighbours = state_graph.get_incoming_neighbour_name_description(&init_type_name);

                // generate From<T> for reach of these neighbours
                let from_code_list = match neighbours
                    .iter()
                    .map(|(from_type, conversion_function)| {
                        generate_from(
                            from_type,
                            &init_type_name,
                            conversion_function,
                        )
                    })
                    .collect::<Result<Vec<TokenStream>, TokenStream>>() {
                        Ok(v) => v,
                        Err(e) => return e,
                    };
                
                quote::quote! {
                    #(#from_code_list)*
                }.into()
            }
        }
        Err(e) => {
            to_error(&format!("Failed to parse the state graph file: {:?}", e)).into()
        }
    }
}