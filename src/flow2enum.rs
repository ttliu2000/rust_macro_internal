use proc_macro::TokenStream;
use syn::{ItemEnum, parse_macro_input};
use quote::quote;

use crate::utils::*;
use parser_lib::mermaid_flow::*;
use parser_lib::common::*;
use crate::init_args::*;

/// Expand a flowchart file into a Rust enum, which represent a graph
pub fn expand(attr: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as InitArgs);
    let path = match get_file_pathbuf(args.get_path()) {
        Ok(n) => n,
        Err(e) => return e,
    };

    let mut s = parse_macro_input!(input as ItemEnum);

    let doc_str = format!("Struct items generated from file '{}'", path.display());
    let doc = create_doc_comment(&doc_str);
    s.attrs.push(doc);

    match parse_flowchart_from_path(path.display().to_string().as_str()) {
        Ok(flow) => {
            let graph = FlowchartToGraph::new()
                .convert(flow.get_stmts());

            if !graph.is_tree() {
                let err_msg = format!(
                    "The flowchart in file '{}' is not a valid tree structure.",
                    path.display()
                );
                return to_error(&err_msg);
            }

            let root = match graph.tree_root() {
                Ok(r) => r,
                Err(e) => {
                    let err_msg = format!(
                        "The flowchart in file '{}' has no valid root node: {:?}",
                        path.display(),
                        e
                    );
                    return to_error(&err_msg);
                }
            };
            if graph.tree_depth(root) > 3 {
                let err_msg = format!(
                    "The flowchart in file '{}' is too deep (more than 3 levels).",
                    path.display()
                );
                return to_error(&err_msg);
            }

            let tree = GraphTreeView::new(&graph, root);

            for child in tree.children(root) {
                let variant_name = tree.node_name(child);
                let doc_str = format!("Variant generated from flowchart node '{}'", variant_name);
                let doc = create_doc_comment(&doc_str);

                let mut bindings = vec![];
                for grand_child in tree.children(child) {
                    let grandchild_text = &graph.node(grand_child).data;
                    bindings.push(syn::Ident::new(strip_quotes(grandchild_text), proc_macro2::Span::call_site()));                 
                }

                let variant_name = syn::Ident::new(variant_name, proc_macro2::Span::call_site());
                if bindings.is_empty() { 
                    let variant = syn::parse_quote! {
                        #doc
                        #variant_name
                    };
                    s.variants.push(variant);
                }
                else {
                    let variant = syn::parse_quote! {
                        #doc
                        #variant_name( #(#bindings),* )
                    };
                    s.variants.push(variant);
                }
            }

            TokenStream::from(quote! { #s })
        }
        Err(e) => {
            let err_msg = format!("Failed to parse flowchart file '{}': {:?}", path.display(), e);
            to_error(&err_msg)
        }
    }
}