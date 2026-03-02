use parser_lib::csv::*;
use syn::Ident;
use crate::init_args::*;
use crate::utils::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;
use crate::csv2type::shared;

fn gen_impl_to_vec(
    struct_name: &Ident,
    file_name: &str,
    fields: &Vec<(Ident, (InferredType, bool))>,
    column_names: &Vec<&str>,
) -> TokenStream {
    let fields_with_index = fields
        .iter()
        .enumerate()
        .map(|(index, (field_ident, infer))| (field_ident.clone(), *infer, index))
        .collect::<Vec<_>>();

    shared::gen_impl_to_vec(struct_name, file_name, &fields_with_index, column_names)
}

pub fn expand(args : InitArgs, mut s :ItemStruct ) -> TokenStream {
    let path = match get_file_pathbuf(args.get_path()) {
        Ok(n) => n,
        Err(e) => return e.into(),
    };

    let doc_str = format!("Struct items generated from file '{}'", path.display());
    let doc = create_doc_comment(&doc_str);
    s.attrs.push(doc);

    match parse_csv_file(path.display().to_string().as_str()) {
        Ok(csv_file) => {
            let column_count = csv_file.get_column_count();
            let mut fields = vec![];
            for col_index in 0..column_count {
                let column_cells = csv_file.get_column(col_index);
                let infered_type = infer_column(column_cells);
                let field_type_str = rust_type(infered_type.0, infered_type.1);

                let excel_index = col_index + 1;
                let field_name = format!("field_{}", excel_index);
                let field_ident = syn::Ident::new(&field_name, proc_macro2::Span::call_site());
                let field_type= match string_to_type(&field_type_str) {
                    Ok(t) => t,
                    Err(err) => return err.into(),
                };

                // add field_ident and field_type to fields
                fields.push((field_ident.clone(), infered_type));

                let doc_str = format!(
                    "Field generated from CSV column {} with inferred type '{}'",
                    col_index + 1,
                    field_type_str
                );
                let doc = create_doc_comment(&doc_str);

                let field = syn::parse_quote! {
                    #doc
                    #field_ident: #field_type
                };

                if let Err(err) = push_named_field(&mut s, field) {
                    return err.into();
                }
            }

            let column_names = csv_file.get_header_names().iter()
                                                .map(|s| to_rust_var_name(s, "field"))
                                                .collect::<Vec<_>>();
            let column_names_refs: Vec<&str> = column_names.iter().map(|s| s.as_str()).collect();
            let struct_name = &s.ident;
            let file_path = path.display().to_string();
            let impl_block = gen_impl_to_vec(struct_name, &file_path, &fields, &column_names_refs);

            quote! { #s
                    #impl_block
            }
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