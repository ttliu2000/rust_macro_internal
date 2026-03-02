use parser_lib::common::*;
use parser_lib::csv::InferredType;
use parser_lib::json::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::utils::*;

fn scalar_json_converter_name(inferred_type: &InferredType, for_json_value: bool) -> &'static str {
    match (inferred_type, for_json_value) {
        (InferredType::Bool, true) => "json_to_bool_value",
        (InferredType::Int, true) => "json_to_int_value",
        (InferredType::UInt, true) => "json_to_uint_value",
        (InferredType::Float, true) => "json_to_float_value",
        (InferredType::DateTime, true) => "json_to_datetime_value",
        (InferredType::String, true) => "json_to_string_value",
        (InferredType::Bool, false) => "json_to_bool",
        (InferredType::Int, false) => "json_to_int",
        (InferredType::UInt, false) => "json_to_uint",
        (InferredType::Float, false) => "json_to_float",
        (InferredType::DateTime, false) => "json_to_datetime",
        (InferredType::String, false) => "json_to_string",
    }
}

pub fn get_conversion_function_name<F>(
    json_type: &JsonType,
    resolve_object_type_name: &F,
) -> Result<TokenStream, TokenStream>
where
    F: Fn(&str) -> String,
{
    match json_type {
        JsonType::Scalar(inferred_type) => {
            let name = scalar_json_converter_name(inferred_type, true);
            let function_name = Ident::new(name, proc_macro2::Span::call_site());

            Ok(quote! {
                #function_name
            }
            .into())
        }
        JsonType::Optional(inner_type) => {
            if inner_type.is_scalar() {
                let inferred_type = match inner_type.as_ref() {
                    JsonType::Scalar(t) => t,
                    _ => return Err(to_error("unexpected non-scalar type in Optional").into()),
                };

                let name = scalar_json_converter_name(inferred_type, false);
                let function_name = Ident::new(name, proc_macro2::Span::call_site());

                Ok(quote! {
                    #function_name
                }
                .into())
            } else {
                let inner_conversion_function =
                    get_conversion_function_name(inner_type, resolve_object_type_name)?;
                Ok(quote! {
                    |v: &parser_lib::json::Value| {
                        if v.is_null() {
                            None
                        } else {
                            Some( #inner_conversion_function( v ) )
                        }
                    }
                }
                .into())
            }
        }
        JsonType::Array(n) => {
            let item_conversion_function = get_conversion_function_name(n, resolve_object_type_name)?;
            Ok(quote! {
                |v: &parser_lib::json::Value| {
                    json_to_array( v, #item_conversion_function ).unwrap()
                }
            }
            .into())
        }
        JsonType::Object(n, _m) => {
            let type_name = resolve_object_type_name(n);
            let type_name_ident = Ident::new(&type_name, proc_macro2::Span::call_site());
            let function_name = Ident::new("from_json_value", proc_macro2::Span::call_site());
            Ok(quote! {
                #type_name_ident::#function_name
            }
            .into())
        }
    }
}

pub fn create_from_json_string_function(fn_name_ident: &Ident) -> TokenStream {
    let function_name = Ident::new("from_json_string", proc_macro2::Span::call_site());
    let function = quote! {
        pub fn #function_name( json_str: &str ) -> Self {
            let json_value = parser_lib::json::parse_json( json_str ).expect("failed to parse json string");
            Self::#fn_name_ident( &json_value )
        }
    };

    function.into()
}

pub fn create_conversion_function<F>(
    fields: Vec<&Ident>,
    type_info_list: &Vec<JsonType>,
    resolve_object_type_name: &F,
) -> Result<TokenStream, TokenStream>
where
    F: Fn(&str) -> String,
{
    let function_name = Ident::new("from_json", proc_macro2::Span::call_site());
    let parameters: Result<Vec<TokenStream>, TokenStream> = fields
        .iter()
        .zip(type_info_list.iter())
        .map(|(field, ty)| -> Result<TokenStream, TokenStream> {
            Ok(match ty {
                JsonType::Array(n) => {
                    let func_name = get_conversion_function_name(n, resolve_object_type_name)?;
                    quote! {
                        json_to_array(&json_value[ stringify!(#field) ], #func_name ).unwrap()
                    }
                }
                JsonType::Optional(n) => {
                    if n.is_scalar() {
                        let inferred_type = match n.as_ref() {
                            JsonType::Scalar(t) => t,
                            _ => {
                                return Err(to_error("unexpected non-scalar type in Optional").into())
                            }
                        };

                        let name = scalar_json_converter_name(inferred_type, false);
                        let func_name = Ident::new(name, proc_macro2::Span::call_site());

                        quote! {
                            #func_name( &json_value[ stringify!(#field) ] )
                        }
                    } else {
                        let func_name = get_conversion_function_name(n, resolve_object_type_name)?;
                        quote! {
                            Some( #func_name( &json_value[ stringify!(#field) ] ) )
                        }
                    }
                }
                JsonType::Object(n, _m) => {
                    let type_name = resolve_object_type_name(n);
                    let type_name_ident = Ident::new(&type_name, proc_macro2::Span::call_site());
                    quote! {
                        #type_name_ident::#function_name( json_value[ stringify!(#field) ].get_object().unwrap() )
                    }
                }
                JsonType::Scalar(_) => {
                    let func_name = get_conversion_function_name(ty, resolve_object_type_name)?;
                    quote! {
                        #func_name( &json_value[ stringify!(#field) ] )
                    }
                }
            })
        })
        .collect();

    let parameters = parameters?;

    let from_value_conversion = quote! {
        pub fn from_json_value( json_value: &parser_lib::json::Value ) -> Self {
            Self::from_json(json_value.get_object().expect("expect a json object"))
        }
    };

    let from_str_function = create_from_json_string_function(&function_name);

    let function = quote! {
        #from_value_conversion

        pub fn #function_name( json_value: &parser_lib::json::Object ) -> Self {
            Self::new(
                #(#parameters),*
            )
        }

        #from_str_function
    };

    Ok(function.into())
}

pub fn create_impl_block<F>(
    struct_name: &Ident,
    field_info_list: &Vec<(Ident, syn::Type)>,
    json_type_list: &Vec<JsonType>,
    resolve_object_type_name: &F,
) -> Result<TokenStream, TokenStream>
where
    F: Fn(&str) -> String,
{
    let new_function_name = Ident::new("new", struct_name.span());
    let new_function = create_new_function(&new_function_name, field_info_list);

    let fields = field_info_list
        .iter()
        .map(|(field_name, _)| field_name)
        .collect();
    let conversion_function =
        create_conversion_function(fields, json_type_list, resolve_object_type_name)?;
    let impl_block = quote! {
        impl #struct_name {
            #new_function

            #conversion_function
        }
    };

    Ok(impl_block.into())
}

pub fn default_object_type_name(name: &str) -> String {
    pascal(name)
}