use parser_lib::csv::*;
use proc_macro2::TokenStream;
use syn::Ident;
use syn::ItemStruct;
use quote::quote;
use syn::Type;

use crate::init_args::*;
use crate::utils::*;
use crate::csv2type::*;
use crate::csv2type::shared;

fn gen_impl_to_vec(
    struct_name: &Ident,
    file_name: &str,
    fields: &Vec<(Ident, (InferredType, bool), usize)>,
    column_names: &Vec<&str>,
) -> TokenStream {
    shared::gen_impl_to_vec(struct_name, file_name, fields, column_names)
}

/// process the type as return type, if T is primitive type, return T, otehrwise, return &T
fn process_type_as_return(ty: &Type) -> Type {
    if is_primitive_scalar(ty) {
        ty.clone()
    }
    else {
        create_type_ref(ty)
    }
}

/// create boolean expression for filter function in lookup function, for example, 
/// if the field_names are ["name", "age"], and the target_names are ["myname", "age"], 
/// then it will generate an expression like this: `x.name == myname && x.age == age
fn create_boolean_expr(struct_name: &str, field_names: &Vec<String>, target_names: &Vec<String>) -> Result<TokenStream, TokenStream> {
    if field_names.len() != target_names.len() {
        return Err(to_error("field_names and target_names must have the same length").into());
    }

    let x_ident = Ident::new(struct_name, proc_macro2::Span::call_site());
    let mut exprs = vec![];

    for (field_name, target_name) in field_names.iter().zip(target_names.iter()) {
        let field_ident = Ident::new(&format!("get_{field_name}"), proc_macro2::Span::call_site());
        let target_value = ident_from_str(target_name);

        let expr = quote! {
            #x_ident.#field_ident() == #target_value
        };

        exprs.push(expr);
    }

    // combine all expr with &&
    let combined_expr = exprs.into_iter().reduce(|acc, expr| {
        quote! {
            #acc && #expr
        }
    });

    Ok(combined_expr.unwrap_or_else(|| to_error("cannot generate bool expression").into() ))
}

fn create_output_value_expr(struct_name: &str, field_names: &Vec<String>) -> TokenStream {
    let x_ident = Ident::new(struct_name, proc_macro2::Span::call_site());
    let mut exprs = vec![];

    for field_name in field_names.iter() {
        let field_ident = Ident::new(&format!("get_{field_name}"), proc_macro2::Span::call_site());

        let expr = quote! {
            #x_ident.#field_ident()
        };

        exprs.push(expr);
    }

    // if only one field, return that field, otherwise return a tuple
    if exprs.len() == 1 {
        exprs.into_iter().next().unwrap_or_else(|| to_error("cannot generate output expression").into())
    }
    else {
        quote! {
            (#(#exprs),*)
        }
    }
}

/// generate lookup functions based on the column csv file, for example, if the column csv file has a row like "lookup_name_age,in,in,out", then it will generate a function like this:
/// ```
/// impl StructName {
///    pub fn lookup_name_age(data:&Vec<Self>, name, age) -> Vec<score>
/// ```
fn gen_lookup_functions(struct_name: &Ident, col_csv_file: &CSVFile, field_types: &Vec<Option<Type>>) -> Result<TokenStream, TokenStream> {
    let lookup_function_names = get_lookup_function_names(col_csv_file);

    let mut functions = vec![];
    for (function_name, ins, outs) in lookup_function_names {
        let function_ident = Ident::new(&function_name, proc_macro2::Span::call_site());

        let mut input_params = vec![];
        let mut param_names = vec![];
        for (index, header) in ins {
            let param_name = to_rust_var_name(&header, "param");
            param_names.push(param_name.clone());

            let param_ident = Ident::new(&param_name, proc_macro2::Span::call_site());

            // the field_types and index are from the csv data file, 
            // the index variable is from col_lookup csv, it's index is offset by +1 from csv data file
            let param_type = match field_types.get(index - 1).and_then(|x| x.as_ref()) {
                Some(t) => t,
                None => return Err(to_error(&format!("function '{}' Index {} '{}' out of bounds for input field_types", &function_name, index, &header)).into()),
            };
            let param_type2 = if is_primitive_scalar(param_type) {
                param_type.clone()
            }
            else {
                create_type_ref(param_type)
            };
            input_params.push(quote! { #param_ident: #param_type2 });
        }

        let mut output_types = vec![];
        let mut output_field_names = vec![];
        for (index, header) in outs {
            let output_field_name = to_rust_var_name(&header, "output");
            output_field_names.push(output_field_name.clone());
            // the field_types and index are from the csv data file, 
            // the index variable is from col_lookup csv, it's index is offset by +1 from csv data file
            let output_type = match field_types.get(index - 1).and_then(|x| x.as_ref()) {
                Some(t) => t,
                None => return Err(to_error(&format!("function '{}' Index {} '{}' out of bounds for output field_types", &function_name, index, header)).into()),
            };
            output_types.push(output_type);
        }

        // For simplicity, we assume the output is a single type. If there are multiple outputs, you may want to return a tuple or a struct.
        let output_type = if output_types.len() == 1 {
            process_type_as_return(output_types[0])
        } else {
            // If there are multiple outputs, we can create a tuple type
            create_tuple_type(output_types)?
        };

        let var_name_in_lambda = "x";
        let bool_expr = create_boolean_expr(var_name_in_lambda, &param_names, &param_names)?;
        let get_value_expr = create_output_value_expr(var_name_in_lambda, &output_field_names);

        let lambda_var_name_ident = ident_from_str(var_name_in_lambda);
        let function = quote! {
            pub fn #function_ident(data: &Vec<Self>, #(#input_params),*) -> Vec<#output_type> {
                // Function body can be implemented based on the actual lookup logic
                let r = data.iter()
                            .filter(| #lambda_var_name_ident | #bool_expr )
                            .map(| #lambda_var_name_ident | #get_value_expr )
                            .collect::<Vec<_>>();
                r
            }
        };

        functions.push(function);
    }

    Ok(quote! {
        impl #struct_name {
            #(#functions)*
        }
    })
}

pub fn expand(args : InitArgs2LitStr, mut s :ItemStruct ) -> TokenStream {
    let path = match get_file_pathbuf(args.get_path()) {
        Ok(n) => n,
        Err(e) => return e.into(),
    };

    let col_file_path = match get_file_pathbuf(args.get_tag()) {
        Ok(n) => n,
        Err(e) => return e.into(),
    };

    let doc_str = format!("Struct items generated from file '{}'", path.display());
    let doc = create_doc_comment(&doc_str);
    s.attrs.push(doc);

    match (parse_csv_file(path.display().to_string().as_str()), parse_csv_file(col_file_path.display().to_string().as_str())) {
        (Ok(csv_file), Ok(col_csv_file))=> {
            let included_headers = get_included_headers(&col_csv_file);

            let column_count = csv_file.get_column_count();
            let mut fields = vec![];
            let mut column_names = vec![];
            let mut field_types = vec![];
            for col_index in 0..column_count {
                let current_col_name = csv_file.get_header_name(col_index);
                let Some(current_col_name) = current_col_name else {
                    field_types.push(None); // push none to keep index align with csv data file
                    continue;
                };

                if !included_headers.contains(&current_col_name) {
                    field_types.push(None); // push none to keep index align with csv data file
                    continue;
                }
                column_names.push(to_rust_var_name(&current_col_name, "field"));

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

                field_types.push(Some(field_type.clone()));

                // add field_ident and field_type to fields
                fields.push((field_ident.clone(), infered_type, col_index));

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

            let lookup_block = match gen_lookup_functions(&s.ident, &col_csv_file, &field_types) {
                Ok(block) => block,
                Err(e) => return e,
            };

            let column_names_refs: Vec<&str> = column_names.iter().map(|s| s.as_str()).collect();
            let struct_name = &s.ident;
            let file_path = path.display().to_string();
            let impl_block = gen_impl_to_vec(struct_name, &file_path, &fields, &column_names_refs);

            quote! { #s
                    #impl_block

                    #lookup_block
            }
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