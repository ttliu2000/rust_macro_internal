use proc_macro2::TokenStream;
use syn::Ident;
use syn::ItemEnum;
use syn::Type;

use crate::init_args::*;
use crate::utils::*;
use parser_lib::csv::*;
use crate::csv2type::*;

/// get function name and output column index and type from the column csv file
fn gen_function_name_outs(csv_file:&CSVFile, col_csv_file:&CSVFile) -> Vec<(String, Vec<(usize, Type)>)> {
    let mut result = vec![];
    for row in col_csv_file.get_data_records() {
        let function_name = row.get_field(0).to_string();
        let _ins = get_header_name_by_cell_value(&col_csv_file.get_header_names(), row, "in");
        let outs = get_header_name_by_cell_value(&col_csv_file.get_header_names(), row, "out");

        // for each out column, find the column index and type in the main csv file
        let mut out_types = vec![];
        for (col_index, _col_name) in outs {
            let col_type = infer_column_type(csv_file, col_index -1);
            if col_type.is_none() {
                continue;
            }
            out_types.push((col_index, col_type.unwrap()));
        }

        result.push((function_name, out_types));
    }

    result
}

fn generate_match_arms(enum_name:&Ident, variants : &Vec<Ident>, csv_file: &CSVFile, outs : &Vec<(usize, Type)>) -> Vec<TokenStream> {
    let mut arms = vec![];
    let len = variants.len();
    for i in 0..len {
        let variant = &variants[i];
        let mut output_exprs = vec![];
        for (col_index, _col_type) in outs {
            let cell_value = csv_file.get_cell_value(i+1, *col_index -1);   // i+1 because the 1st row is header
            let values = csv_file.get_column_data(*col_index - 1);
            let col_name = csv_file.get_header_name(*col_index - 1).unwrap_or_else(|| format!("Column {}", col_index));
            let (infer_type, option) = infer_column(values);
            
            match infer_type { 
                InferredType::Bool => {
                    if cell_value.to_lowercase().trim() == "true" || cell_value.to_lowercase().trim() == "1" {
                        if option {
                            output_exprs.push(quote::quote! { Some(true) });
                        }
                        else {
                            output_exprs.push(quote::quote! { true });
                        }
                    } else if cell_value.to_lowercase().trim() == "false" || cell_value.to_lowercase().trim() == "0" {
                        if option {
                            output_exprs.push(quote::quote! { Some(false) });
                        }
                        else {
                            output_exprs.push(quote::quote! { false });
                        }
                    } else {
                        if option { 
                            output_exprs.push(quote::quote! { None });
                        }
                        else {
                            let err_msg = format!(
                                "Failed to parse boolean value '{}' in column index {} / {col_name}",
                                cell_value,
                                col_index
                            );
                            return vec![to_error(&err_msg).into()];
                        }
                    }
                }
                InferredType::Int => {
                    match parse_i64_literal(cell_value.trim()) {
                        Ok(v) => {
                            if option {
                                output_exprs.push(quote::quote! { Some(#v) });
                            }
                            else {
                                output_exprs.push(quote::quote! { #v });
                            }
                        }
                        Err(_) => {
                            if option { 
                                output_exprs.push(quote::quote! { None });
                            }
                            else {
                                let err_msg = format!(
                                    "Failed to parse integer value '{}' in column index {} / {col_name}",
                                    cell_value,
                                    col_index
                                );
                                return vec![to_error(&err_msg).into()];
                            }
                        }
                    }
                }
                InferredType::UInt => {
                    match parse_u64_literal(cell_value.trim()) {
                        Ok(v) => {
                            if option {
                                output_exprs.push(quote::quote! { Some(#v) });
                            }
                            else {
                                output_exprs.push(quote::quote! { #v });
                            }
                        }
                        Err(_) => {
                            if option { 
                                output_exprs.push(quote::quote! { None });
                            }
                            else {
                                let err_msg = format!(
                                    "Failed to parse unsigned integer value '{}' in column index {} / {col_name}",
                                    cell_value,
                                    col_index
                                );
                                return vec![to_error(&err_msg).into()];
                            }
                        }
                    }
                }
                InferredType::Float => {
                    match cell_value.trim().parse::<f64>() {
                        Ok(v) => {
                            if option {
                                output_exprs.push(quote::quote! { Some(#v) });
                            }
                            else {
                                output_exprs.push(quote::quote! { #v });
                            }
                        }
                        Err(_) => {
                            if option { 
                                output_exprs.push(quote::quote! { None });
                            }
                            else {
                                let err_msg = format!(
                                    "Failed to parse float value '{}' in column index {} / {col_name}",
                                    cell_value,
                                    col_index
                                );
                                return vec![to_error(&err_msg).into()];
                            }
                        }
                    }
                }
                InferredType::DateTime => {
                    let err_msg = format!(
                        "DateTime type is not supported for column index {} in lookup function / {col_name}",
                        col_index
                    );
                    return vec![to_error(&err_msg).into()];
                }
                InferredType::String => {
                    let cell_value_literal = syn::LitStr::new(&cell_value, proc_macro2::Span::call_site());
                    if option {
                        output_exprs.push(quote::quote! { Some(#cell_value_literal.to_string()) });
                    }
                    else {
                        output_exprs.push(quote::quote! { #cell_value_literal.to_string() });
                    }
                }
            }
        }

        let arm = quote::quote! {
            #enum_name::#variant => ( #(#output_exprs),* )
        };
        arms.push(arm);
    }

    arms
}

fn gen_lookup_functions(enum_name:&Ident, csv_file:&CSVFile, col_csv_file:&CSVFile, variants : &Vec<Ident>) -> TokenStream {
    let lookup_function_names = gen_function_name_outs(csv_file, col_csv_file);
    let mut functions = vec![];
    for (lookup_name, outs) in lookup_function_names {
        let mut output_types = vec![];
        for (_index, ty) in &outs {
            let y = quote::quote! { #ty };
            output_types.push(y);
        }

        let arms = generate_match_arms(enum_name, variants, csv_file, &outs);
        let function_name = Ident::new(&lookup_name, proc_macro2::Span::call_site());
        let function = quote::quote! {
            pub fn #function_name(&self) -> (#(#output_types),*)  {
                match self {
                    #(#arms),*
                }
            }
        };

        functions.push(function);
    }

    quote::quote! {
        #(#functions)*
    }
}

pub fn expand(args : InitArgs3_2LitStr, mut s :ItemEnum ) -> TokenStream {
    let path = match get_file_pathbuf(args.get_path()) {
        Ok(n) => n,
        Err(e) => return e.into(),
    };

    let col_file_path = match get_file_pathbuf(args.get_tag()) {
        Ok(n) => n,
        Err(e) => return e.into(),
    };
    

    match (parse_csv_file(&path.display().to_string()), parse_csv_file(&col_file_path.display().to_string())) {
        (Ok(csv_file), Ok(col_csv_file)) => {
            let col_name = args.get_tag2().to_string();

            let col_index = match csv_file.find_column_index_by_name(&col_name) {
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

            let col_data = csv_file.get_column_data(col_index);
            let mut variants = vec![];
            for name in col_data {
                let enum_variant_name = to_rust_enum_variant_name(name);
                let ident = Ident::new(&enum_variant_name, proc_macro2::Span::call_site());
                variants.push(ident.clone());

                s.variants.push(syn::Variant {
                    ident,
                    fields: syn::Fields::Unit,
                    discriminant: None,
                    attrs: Vec::new(),
                });
            }

            // generate lookup functions
            let type_name = &s.ident;
            let lookup_functions = gen_lookup_functions(type_name, &csv_file, &col_csv_file, &variants);

            quote::quote! {
                #s

                impl #type_name {
                    #lookup_functions
                }
            }.into()
        }
        (Err(e), _) => {
            let err_msg = format!(
                "Failed to parse CSV file '{}': {:?}",
                path.display(),
                e
            );
            
            to_error(&err_msg).into()
        }
        (_, Err(e)) => {
            let err_msg = format!(
                "Failed to parse CSV file '{}': {:?}",
                 col_file_path.display(),
                e
            );
            
            to_error(&err_msg).into()
        }
    }
}