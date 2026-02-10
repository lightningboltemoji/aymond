use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, meta::parser, parse_macro_input};

use crate::{
    definition::{ItemAttribute, ItemDefinition, NestedItemDefinition},
    item::create_item,
    nested_item::create_nested_item,
    query_builder::create_query_builder,
    table::create_table,
};

mod definition;
mod get_builder;
mod item;
mod nested_item;
mod query_builder;
mod table;

#[proc_macro_attribute]
pub fn aymond(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);

    let mut item: bool = false;
    let mut nested_item: bool = false;
    let mut table: bool = false;
    let arg_parser = parser(|meta| {
        if meta.path.is_ident("item") {
            item = true;
            Ok(())
        } else if meta.path.is_ident("nested_item") {
            nested_item = true;
            Ok(())
        } else if meta.path.is_ident("table") {
            table = true;
            Ok(())
        } else {
            Err(meta.error("Unsupported attribute"))
        }
    });
    parse_macro_input!(args with arg_parser);

    let chunks: Vec<proc_macro2::TokenStream> = match (item, nested_item, table) {
        (false, false, false) => panic!("Must specify attribute e.g. #[aymond(item)]"),
        (true, true, _) => panic!("Can't specify both item and nested_item"),
        (false, _, true) => panic!("Can't specify table without item"),
        (_, true, _) => vec![create_nested_item(&mut input)],
        (true, _, false) => vec![create_item(&mut input)],
        (true, _, true) => {
            let item = create_item(&mut input);
            let table = create_table(&input);
            vec![quote!(#item), quote!(#table)]
        }
    };

    quote! {
        #( #chunks )*
    }
    .into()
}
