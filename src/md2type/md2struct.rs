use parser_lib::markdown_lang::load_md_file;
use proc_macro2::TokenStream;
use syn::ItemStruct;
use quote::quote;

use crate::init_args::*;
use crate::utils::*;

fn generate_struct_from_var_names_types(mut s: ItemStruct, var_names_types: impl IntoIterator<Item = (String, String)>) -> TokenStream {
    for (name, ty) in var_names_types.into_iter() {
        let name_ident = ident_from_str(&name);
        match string_to_type(&ty) {
            Ok(ty) => {
                // create named field 
                let field = syn::parse_quote! {
                    #name_ident : #ty
                };

                match push_named_field(&mut s, field) {
                    Ok(_) => {},
                    Err(e) => return e.into(),
                }
            }
            Err(e) => {
                return e;
            }
        }
    }

    quote! { 
        #s
    }
}

pub fn expand(args : InitArgs2LitStr, mut s :ItemStruct ) -> TokenStream {
    let path = match get_file_pathbuf(args.get_path()) {
        Ok(n) => n,
        Err(e) => return syn::Error::new_spanned(&s, e.to_string()).into_compile_error(),
    };

    let doc_str = format!("Struct items generated from file '{}'", path.display());
    let doc = create_doc_comment(&doc_str);
    s.attrs.push(doc);

    let header_text = args.get_tag().value();

    match load_md_file(path.display().to_string().as_str()) {
        Ok(md_file) => {
            if let Some(header) = md_file.get_headers().into_iter().find(|x| x.get_text() == header_text) {
                // find table after header
                let text_lines = md_file.get_all_text_lines_after_header(&header_text, header.get_level());
                if let Some(table) = text_lines.into_iter().find_map(|x| x.get_table()) {
                    if let Ok(var_names) = table.get_col_data(0) {
                        if let Ok(var_types) = table.get_col_data(1) {
                            let var_names_types = var_names.into_iter()
                                                        .map(|x| x.trim().to_string())
                                                        .zip(var_types.into_iter().map(|x| x.trim().to_string()));
                            return generate_struct_from_var_names_types(s, var_names_types).into();
                        }
                        else {
                            let err_str = format!("Cannot get variable types from the second column of the table after header '{}' in markdown file '{}'", header_text, path.display());
                            return syn::Error::new_spanned(s, err_str).into_compile_error();
                        }
                    }  
                    else {
                        let err_str = format!("Cannot get variable names from the first column of the table after header '{}' in markdown file '{}'", header_text, path.display());
                        return syn::Error::new_spanned(s, err_str).into_compile_error();
                    }                  
                }
                
                todo!("support multiple tables after header, currently only the first table is supported");
            }
            else {
                let err_str = format!("Cannot find header '{}' in markdown file '{}'", header_text, path.display());
                return syn::Error::new_spanned(s, err_str).into_compile_error();
            }            
        }
        Err(e) => {
            let err_str = format!("Error parsing/loading markdown file '{}': {:?}", path.display(), e);
            return syn::Error::new_spanned(s, err_str).into_compile_error();
        }
    }
}