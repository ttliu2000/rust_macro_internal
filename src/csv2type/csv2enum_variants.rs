use parser_lib::csv::*;
use syn::Ident;
use syn::ItemEnum;
use proc_macro2::TokenStream;

use crate::init_args::*;
use crate::utils::*;

pub fn expand(args : InitArgs2, mut s :ItemEnum ) -> TokenStream {
    let path = match get_file_pathbuf(args.get_path()) {
        Ok(n) => n,
        Err(e) => return e.into(),
    };

     match parse_csv_file(path.display().to_string().as_str()) {
        Ok(csv_file) => {
            let col_name = args.get_tag().to_string();
            let index = match csv_file.find_column_index_by_name(&col_name) {
                Some(i) => i,
                None => {
                    let err_msg = format!(
                        "Column '{}' not found in CSV file '{}'",
                        col_name,
                        path.display()
                    );
                    return to_error(&err_msg).into();
                }
            };
            let doc_str = format!("Add enum items generated from file '{}' column #{} '{}'", path.display(), index, col_name);
            let doc = create_doc_comment(&doc_str);
            s.attrs.push(doc);

            let col_data = csv_file.get_column_data(index);
            
            for name in col_data {
                let enum_variant_name = name.to_string(); //to_rust_enum_variant_name(name);
                let ident = Ident::new(&enum_variant_name, proc_macro2::Span::call_site());

                s.variants.push(syn::Variant {
                    ident,
                    fields: syn::Fields::Unit,
                    discriminant: None,
                    attrs: Vec::new(),
                });
            }

            quote::quote! {
                #s
            }.into()
        }
        Err(e) => {
            let err_msg = format!(
                "Failed to parse CSV file '{}': {:?}",
                path.display(),
                e
            );
            
            to_error(&err_msg).into()
        }
    }
}