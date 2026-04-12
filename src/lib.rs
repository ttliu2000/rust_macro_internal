use proc_macro::TokenStream;

pub (crate) mod utils;
pub (crate) mod ini2type;
pub (crate) mod init_args;
pub (crate) mod csv2type;
pub (crate) mod packet2type;
pub (crate) mod flow2type;

use crate::ini2type::*;
use crate::csv2type::*;
use crate::state2struct::*;
use crate::packet2type::*;
use crate::flow2type::*;

macro_rules! expand_attr_with_parse {
    ($attr:ident as $attr_ty:ty, $item:ident as $item_ty:ty, $expand:path) => {{
        let args = syn::parse_macro_input!($attr as $attr_ty);
        let item = syn::parse_macro_input!($item as $item_ty);

        $expand(args, item).into()
    }};
}

macro_rules! expand_with_parse {
    ($input:ident as $args_ty:ty, $expand:path) => {{
        let args = syn::parse_macro_input!($input as $args_ty);

        $expand(args).into()
    }};
}

#[proc_macro_attribute]
pub fn ini_enum(attr: TokenStream, input: TokenStream) -> TokenStream {
    ini2enum::expand(attr, input)
}

#[proc_macro_attribute]
pub fn packet_struct(attr: TokenStream, item: TokenStream) -> TokenStream {
    packet2struct::expand(attr, item)
}

#[proc_macro_attribute]
pub fn packet_bit_vec(attr: TokenStream, item: TokenStream) -> TokenStream {
    packet_bit_vec::expand(attr, item)
}

#[proc_macro_attribute]
pub fn ini_struct(attr: TokenStream, item: TokenStream) -> TokenStream {
    ini2struct::expand(attr, item)
}

#[proc_macro_attribute]
pub fn ini_enum_str(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_attr_with_parse!(attr as init_args::InitArgs, item as syn::ItemEnum, ini2enum_str::expand)
}

pub (crate) mod flow2enum;

#[proc_macro_attribute]
pub fn flow_enum(attr: TokenStream, item: TokenStream) -> TokenStream {
    flow2enum::expand(attr, item)
}

#[proc_macro_attribute]
pub fn csv_struct(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_attr_with_parse!(attr as init_args::InitArgs, item as syn::ItemStruct, csv2struct::expand)
}

#[proc_macro_attribute]
pub fn csv_struct2(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_attr_with_parse!(attr as init_args::InitArgs2LitStr, item as syn::ItemStruct, csv2struct2::expand)
}

#[proc_macro_attribute]
pub fn csv2enum_variants(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_attr_with_parse!(attr as init_args::InitArgs2, item as syn::ItemEnum, csv2enum_variants::expand)
}

/// Generate a lookup function for an enum based on two columns in a CSV file.
/// the function name genearte is **lookup_size_<enum_name_lowercase>**.
/// The first argument is the path to the CSV file, the 2nd and 3rd arguments are the column names to be used as keys and values respectively.
/// the 4th argument is the enum name.
#[proc_macro]
pub fn csv2lookup(attr: TokenStream) -> TokenStream {
    expand_with_parse!(attr as init_args::InitArgs4, csv2lookup::expand)
}

/// Generate a lookup function for an enum based column in a CSV file.
/// and the 2nd file is to define lookup table and support look from the enum variant to the value in the CSV file.
#[proc_macro_attribute]
pub fn csv2enum_lookup(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_attr_with_parse!(attr as init_args::InitArgs3_2LitStr, item as syn::ItemEnum, csv2enum_lookup::expand)
}

/// Generate a HashMap from two columns in a CSV file.
/// The first argument is the path to the CSV file, the 2nd and 3rd arguments are the column names to be used as keys and values respectively.
#[proc_macro]
pub fn csv2hash(attr: TokenStream) -> TokenStream {
    expand_with_parse!(attr as init_args::InitArgs3, csv2hash::expand)
}

pub (crate) mod json2struct;

#[proc_macro]
pub fn json_struct(input: TokenStream) -> TokenStream {
    expand_with_parse!(input as init_args::InitArgs2, crate::json2struct::json2struct::expand)
}

#[proc_macro]
pub fn json_struct2(input: TokenStream) -> TokenStream {
    expand_with_parse!(input as init_args::InitArgs3_2LitStr, crate::json2struct::json2struct2::expand)
}

pub (crate) mod state2struct;

#[proc_macro_attribute]
pub fn state_struct(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_attr_with_parse!(attr as init_args::InitArgs, item as syn::ItemStruct, state2struct::expand)
}

#[proc_macro_attribute]
pub fn state_struct_trait(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_attr_with_parse!(attr as init_args::InitArgs, item as syn::ItemStruct, state2struct::expand_with_trait)
}

/// the state diagram shows the mapping conversion among themselves. this macro will generate conversion code to implement these conversions.
/// the state in the diagram represent a type, and the edge represent the conversion function between two types.
#[proc_macro]
pub fn state_type_mapping(input: TokenStream) -> TokenStream {
    expand_with_parse!(input as init_args::InitArgs2, expand_type_mapping)
}

pub (crate) mod sequence_diagram;

#[proc_macro_attribute]
pub fn sequence2function(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_attr_with_parse!(attr as init_args::InitArgs, item as syn::ItemFn, sequence_diagram::sequence2function::expand)
}

pub (crate) mod md2type;

#[proc_macro_attribute]
pub fn md2struct(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_attr_with_parse!(attr as init_args::InitArgs2LitStr, item as syn::ItemStruct, md2type::md2struct::expand)
}

#[proc_macro]
pub fn flow2logic(attr: TokenStream) -> TokenStream {
    expand_with_parse!(attr as init_args::InitArgs4, flow2logic::expand)
}