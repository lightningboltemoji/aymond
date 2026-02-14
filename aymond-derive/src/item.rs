use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Expr, Ident, Type, parse_quote};

use crate::{ItemAttribute, ItemDefinition};

pub fn create_item(input: &mut DeriveInput) -> (TokenStream, ItemDefinition) {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let def = ItemDefinition::new(input, false);
    let name = &input.ident.clone();

    let mut key_scalar_type: Vec<Expr> = vec![];
    key_scalar_type.push(def.hash_key.as_ref().unwrap().scalar_type());
    def.sort_key
        .iter()
        .map(|e| e.scalar_type())
        .for_each(|e| key_scalar_type.push(e));

    let mut attr_ident: Vec<Ident> = vec![];
    let mut attr_name: Vec<String> = vec![];
    let mut attr_insert_map: Vec<TokenStream> = vec![];
    let mut attr_unboxer: Vec<Expr> = vec![];
    let mut attr_ty: Vec<Type> = vec![];

    let mut append = |i: &ItemAttribute| {
        let ident = &i.ident;
        let insert_map = i.insert_into_map(&parse_quote!(self.#ident), &parse_quote!(map));
        let name = i.attr_name.clone();
        let unboxer = if &i.ty == i.ty_non_option() {
            i.from_attribute_value(&parse_quote!(map.get(#name).unwrap()))
        } else {
            let exp = i.from_attribute_value(&parse_quote!(e));
            parse_quote!(map.get(#name).and_then(|e| #exp))
        };

        attr_insert_map.push(insert_map);
        attr_unboxer.push(unboxer);
        attr_ident.push(i.ident.clone());
        attr_ty.push(i.ty.clone());
        attr_name.push(name);
    };

    let has_sort_key = def.sort_key.is_some();
    def.hash_key.iter().for_each(&mut append);
    def.sort_key.iter().for_each(&mut append);
    def.other_attributes.iter().for_each(&mut append);

    let key_attr_name = &attr_name[0..(if has_sort_key { 2 } else { 1 })];
    let key_type: Vec<Expr> = {
        let mut v = vec![parse_quote! {#aws_sdk_dynamodb::types::KeyType::Hash}];
        if has_sort_key {
            v.push(parse_quote! {#aws_sdk_dynamodb::types::KeyType::Range});
        }
        v
    };

    let item = quote! {
        #[derive(Debug, PartialEq)]
        #input

        impl From<&::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>> for #name {
            fn from(map: &::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>) -> Self {
                #name {
                    #( #attr_ident: #attr_unboxer ),*
                }
            }
        }

        impl Into<::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>> for #name {
            fn into(self) -> ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue> {
                let mut map = ::std::collections::HashMap::new();
                #(
                    #attr_insert_map
                )*
                map
            }
        }

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
