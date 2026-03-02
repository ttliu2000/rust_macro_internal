use parser_lib::csv::*;
use syn::Type;

use crate::utils::string_to_type;

pub mod csv2struct;
pub mod csv2hash;
pub mod csv2enum_variants;
pub mod csv2lookup;
pub mod csv2struct2;
pub mod csv2enum_lookup;
pub mod shared;

pub (self) fn get_included_headers(col_csv_file: &CSVFile) -> Vec<String> {
    col_csv_file.get_header_names()
                .into_iter()
                .filter(|name| !name.trim().is_empty())
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
}

/// Generate lookup functions based on the configuration in the column CSV file.
/// the column CSV file should have at least 3 columns: function name, 
/// "in" column and "out" column. 
/// * the "in" column is used to find the corresponding header in the main CSV file as input parameter, 
/// * the "out" column is used to find the corresponding header in the main CSV file as output parameter.
/// return values is a vector of (function name, input parameters, output parameters), 
/// where input parameters and output parameters are vectors of (column index, column name) tuples.
pub (self) fn get_lookup_function_names(col_csv_file: &CSVFile) -> Vec<(String, Vec<(usize, String)>, Vec<(usize, String)>)> {
    let mut result = vec![];

    let headers = col_csv_file.get_header_names();
    for row in col_csv_file.get_data_records() {
        let function_name = row.get_field(0).to_string();
        let ins = get_header_name_by_cell_value(&headers, row, "in");
        let outs = get_header_name_by_cell_value(&headers, row, "out");

        result.push((function_name, ins, outs));
    }

    result
}

/// take header names and row data, if that cell data equals parameter in_value, return (header_index, header name) tuple
pub (self) fn get_header_name_by_cell_value(headers: &Vec<&str>, row: &CSVRecord, in_value: &str) -> Vec<(usize, String)> {
    let mut result = vec![];

    for (i, cell) in row.get_fields().iter().enumerate() {
        if cell.trim() == in_value.trim() {
            if let Some(header) = headers.get(i) {
                result.push((i, header.to_string()));
            }
        }
    }

    result
}

/// infer csv file's column type based on the column data, and return the type
pub (self) fn infer_column_type(csv_file:&CSVFile, col_index: usize) -> Option<Type> {
    let column_cells = csv_file.get_column(col_index);
    let infered_type = infer_column(column_cells);
    let field_type_str = rust_type(infered_type.0, infered_type.1);
    let field_type= match string_to_type(&field_type_str) {
                    Ok(t) => Some(t),
                    Err(_err) => None,
                };
    field_type
}

use std::num::ParseIntError;

pub (self) fn parse_i64_literal(s: &str) -> Result<i64, ParseIntError> {
    let s = s.trim();

    // Handle sign
    let (sign, rest) = if let Some(r) = s.strip_prefix('-') {
        (-1i64, r)
    } else if let Some(r) = s.strip_prefix('+') {
        (1i64, r)
    } else {
        (1i64, s)
    };

    // Remove underscores
    let cleaned = rest.replace('_', "");

    // Detect radix
    let (radix, digits) = if let Some(hex) = cleaned.strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
    {
        (16, hex)
    } else if let Some(bin) = cleaned.strip_prefix("0b")
        .or_else(|| cleaned.strip_prefix("0B"))
    {
        (2, bin)
    } else if let Some(oct) = cleaned.strip_prefix("0o")
        .or_else(|| cleaned.strip_prefix("0O"))
    {
        (8, oct)
    } else {
        (10, cleaned.as_str())
    };

    let value = i64::from_str_radix(digits, radix)?;
    Ok(sign * value)
}

pub (self) fn parse_u64_literal(s: &str) -> Result<u64, ParseIntError> {
    let s = s.trim();

    // Remove underscores
    let cleaned = s.replace('_', "");

    // Detect radix
    let (radix, digits) = if let Some(hex) = cleaned.strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
    {
        (16, hex)
    } else if let Some(bin) = cleaned.strip_prefix("0b")
        .or_else(|| cleaned.strip_prefix("0B"))
    {
        (2, bin)
    } else if let Some(oct) = cleaned.strip_prefix("0o")
        .or_else(|| cleaned.strip_prefix("0O"))
    {
        (8, oct)
    } else {
        (10, cleaned.as_str())
    };

    u64::from_str_radix(digits, radix)
}