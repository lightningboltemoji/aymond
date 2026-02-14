use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, Expr, parse_quote};

use crate::{ItemDefinition, marshal};

pub fn create_item(input: &mut DeriveInput) -> (TokenStream, ItemDefinition) {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let def = ItemDefinition::new(input, false);
    let name = format_ident!("{}", def.name);

    let key_itr = || def.hash_key.iter().chain(def.sort_key.iter());
    let key_scalar_type = key_itr().map(|ia| ia.scalar_type()).collect::<Vec<Expr>>();
    let key_attr_name = key_itr()
        .map(|ia| ia.ddb_name.to_string())
        .collect::<Vec<String>>();
    let key_type: Vec<Expr> = {
        let mut v = vec![parse_quote! {#aws_sdk_dynamodb::types::KeyType::Hash}];
        if def.sort_key.is_some() {
            v.push(parse_quote! {#aws_sdk_dynamodb::types::KeyType::Range});
        }
        v
    };

    let from_into = marshal::from_into_item_structure(&def);
    let item = quote! {
        #[derive(Debug, PartialEq)]
        #input
        #from_into

        impl Item for #name {
            fn key_schemas() -> Vec<#aws_sdk_dynamodb::types::KeySchemaElement> {
                vec![
                    #(
                        #aws_sdk_dynamodb::types::KeySchemaElement::builder()
                            .attribute_name(#key_attr_name)
                            .key_type(#key_type)
                            .build()
                            .unwrap()
                    ),*
                ]
            }

            fn key_attribute_defintions() -> Vec<#aws_sdk_dynamodb::types::AttributeDefinition> {
                vec![
                    #(
                        #aws_sdk_dynamodb::types::AttributeDefinition::builder()
                            .attribute_name(#key_attr_name)
                            .attribute_type(#key_scalar_type)
                            .build()
                            .unwrap()
                    ),*
                ]
            }
        }
    };
    (item, def)
}
