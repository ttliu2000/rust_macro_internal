use parser_lib::csv::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::utils::*;

pub(super) fn gen_impl_to_vec(
    struct_name: &Ident,
    file_name: &str,
    fields: &Vec<(Ident, (InferredType, bool), usize)>,
    column_names: &Vec<&str>,
) -> TokenStream {
    let fn_ident = Ident::new("new", struct_name.span());

    let mut fields2 = vec![];
    for (field_ident, (inferred_type, optional), _index) in fields.iter() {
        let field_type_str = rust_type(*inferred_type, *optional);
        let field_type = match string_to_type(&field_type_str) {
            Ok(t) => t,
            Err(err) => return err.into(),
        };
        fields2.push((field_ident.clone(), field_type));
    }

    let mut get_fields = vec![];
    for (field_ident, (inferred_type, optional), index) in fields {
        let func_name = convert_function(inferred_type);
        let index_argument = match create_litint(&index.to_string()) {
            Ok(n) => n,
            Err(e) => return e.into(),
        };

        if let Some(func_name) = func_name {
            let get_fun_ident = Ident::new("get_field", struct_name.span());
            let func_name_ident = Ident::new(&func_name, struct_name.span());
            let get_field = if !*optional {
                quote! {
                    #func_name_ident(record.#get_fun_ident(#index_argument)).unwrap()
                }
            } else {
                quote! {
                    #func_name_ident(record.#get_fun_ident(#index_argument))
                }
            };

            get_fields.push(get_field);
        } else {
            return quote! {
                compile_error!("Cannot find conversion function for field {}", stringify!(#field_ident));
            };
        }
    }

    let new_func = create_new_function(&fn_ident, &fields2);
    let getters = create_getters(&fields2);
    let setters = create_setters(&fields2);

    let getters_with_names = create_getters_with_names(&fields2, column_names);
    let setters_with_names = create_setters_with_names(&fields2, column_names);

    quote! {
        impl #struct_name {
            #new_func

            #getters
            #setters

            #getters_with_names
            #setters_with_names

            pub fn to_typed_vec() -> Vec<Self> {
                let csv_file = parser_lib::csv::parse_csv_file(#file_name)
                    .expect("Failed to parse CSV file");

                let mut result = vec![];
                for record in csv_file.get_data_records() {
                    result.push(
                        #struct_name::new(
                        #(#get_fields),*
                    ));
                }

                result
            }
        }
    }
}