use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Expr, Ident, Type, parse_quote};

use crate::{ItemAttribute, NestedItemDefinition};

pub fn create_nested_item(input: &mut DeriveInput) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let name = &input.ident.clone();
    let def: NestedItemDefinition = input.into();

    let mut attr_ident: Vec<Ident> = vec![];
    let mut attr_name: Vec<String> = vec![];
    let mut attr_boxer: Vec<Expr> = vec![];
    let mut attr_unboxer: Vec<Expr> = vec![];
    let mut attr_ty: Vec<Type> = vec![];

    let append = |i: ItemAttribute| {
        let ident = &i.ident;
        let boxer = i.into_attribute_value(&parse_quote!(self.#ident));

        let name = i.attr_name.clone();
        let unboxer = i.from_attribute_value(&parse_quote!(map.get(#name).unwrap()));

        attr_boxer.push(boxer);
        attr_unboxer.push(unboxer);
        attr_ident.push(i.ident);
        attr_name.push(i.attr_name);
        attr_ty.push(i.ty);
    };

    def.attributes.into_iter().for_each(append);

    quote! {
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
                    map.insert(#attr_name.to_string(), #attr_boxer);
                )*
                map
            }
        }
    }
}
