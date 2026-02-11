use crate::definition::ItemDefinition;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, parse_quote};

pub fn create_get_builder(item: &ItemDefinition) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let query_struct = format_ident!("{}GetItem", &item.name);
    let hash_key_struct = format_ident!("{}GetItemHashKey", &item.name);

    let hash_key_attr_name = &item.hash_key.attr_name;
    let hash_key_ident = &item.hash_key.ident;
    let hash_key_typ = &item.hash_key.typ_ident;
    let hash_key_boxer = &item.hash_key.key_boxer_for(&parse_quote!(self.hk.unwrap()));

    if item.sort_key.is_some() {
        let sort_key_struct = format_ident!("{}GetItemSortKey", &item.name);
        let sort_key_ident = &item.sort_key.as_ref().unwrap().ident;
        let sort_key_attr_name = &item.sort_key.as_ref().unwrap().attr_name;
        let sort_key_typ = &item.sort_key.as_ref().unwrap().typ_ident;

        let sort_key_boxer = &item.hash_key.key_boxer_for(&parse_quote!(self.sk.unwrap()));

        quote! {
            struct #hash_key_struct {
                q: #query_struct,
            }

            struct #query_struct {
                hk: Option<String>,
                sk: Option<#sort_key_typ>,
            }

            impl #query_struct {
                fn new() -> #hash_key_struct {
                    let q = #query_struct { hk: None, sk: None };
                    #hash_key_struct { q }
                }
            }

            impl #hash_key_struct {
                fn #hash_key_ident (mut self, v: impl Into<#hash_key_typ>) -> #sort_key_struct {
                    self.q.hk = Some(v.into());
                    #sort_key_struct { q: self.q }
                }
            }

            struct #sort_key_struct {
                q: #query_struct,
            }

            impl #sort_key_struct {
                fn #sort_key_ident (mut self, sk: impl Into<#sort_key_typ>) -> #query_struct {
                    self.q.sk = Some(sk.into());
                    self.q
                }
            }

            impl Into<::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>> for #query_struct {
                fn into(self) -> ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue> {
                    let mut key_values = ::std::collections::HashMap::new();
                    key_values.insert(#hash_key_attr_name.to_string(), #hash_key_boxer);
                    if self.sk.is_some() {
                        key_values.insert(#sort_key_attr_name.to_string(), #sort_key_boxer);
                    }
                    key_values
                }
            }
        }
    } else {
        quote! {
            struct #hash_key_struct {
                q: #query_struct,
            }

            struct #query_struct {
                hk: Option<String>,
            }

            impl #query_struct {
                fn new() -> #hash_key_struct {
                    let q = #query_struct { hk: None };
                    #hash_key_struct { q }
                }
            }

            impl #hash_key_struct {
                fn #hash_key_ident (mut self, v: impl Into<#hash_key_typ>) -> #query_struct {
                    self.q.hk = Some(v.into());
                    self.q
                }
            }

            impl Into<::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>> for #query_struct {
                fn into(self) -> ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue> {
                    let mut key_values = ::std::collections::HashMap::new();
                    key_values.insert(#hash_key_attr_name.to_string(), #hash_key_boxer);
                    key_values
                }
            }
        }
    }
}
