use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, Ident, parse_quote};

use crate::{ItemAttribute, ItemDefinition};

pub fn from_into_item_structure(item: &ItemDefinition) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);
    let name = format_ident!("{}", item.name);

    let mut fields: Vec<Ident> = vec![];
    let mut insert_maps: Vec<TokenStream> = vec![];
    let mut unboxers: Vec<Expr> = vec![];

    let mut append = |i: &ItemAttribute| {
        let field = i.field.clone();
        let insert_map = i.insert_into_map(&parse_quote!(self.#field), &parse_quote!(map));
        fields.push(field);
        insert_maps.push(insert_map);

        let name = i.ddb_name.clone();
        let unboxer = if !i.is_option {
            i.from_attribute_value(&parse_quote!(map.get(#name).unwrap()))
        } else {
            let exp = i.from_attribute_value(&parse_quote!(e));
            parse_quote!(map.get(#name).and_then(|e| #exp))
        };

        unboxers.push(unboxer);
    };

    item.all_attributes().for_each(&mut append);

    quote! {
        impl From<&::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>> for #name {
            fn from(map: &::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>) -> Self {
                #name {
                    #( #fields: #unboxers ),*
                }
            }
        }

        impl Into<::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>> for #name {
            fn into(self) -> ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue> {
                let mut map = ::std::collections::HashMap::new();
                #(
                    #insert_maps
                )*
                map
            }
        }
    }
}
