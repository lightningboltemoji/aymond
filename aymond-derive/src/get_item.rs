use crate::definition::ItemDefinition;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, parse_quote};

pub fn create_get_builder(item: &ItemDefinition) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let item_struct = format_ident!("{}", &item.name);
    let table_struct = format_ident!("{}Table", &item.name);
    let get_item_struct = format_ident!("{}GetItem", &item.name);
    let hash_key_struct = format_ident!("{}GetItemHashKey", &item.name);

    let hash_key_attr_name = &item.hash_key.attr_name;
    let hash_key_ident = &item.hash_key.ident;
    let hash_key_typ = &item.hash_key.typ_ident;
    let hash_key_boxer = &item.hash_key.key_boxer_for(&parse_quote!(self.hk.unwrap()));

    let builders = if item.sort_key.is_some() {
        let sort_key_struct = format_ident!("{}GetItemSortKey", &item.name);
        let sort_key_ident = &item.sort_key.as_ref().unwrap().ident;
        let sort_key_attr_name = &item.sort_key.as_ref().unwrap().attr_name;
        let sort_key_typ = &item.sort_key.as_ref().unwrap().typ_ident;

        let sort_key_boxer = &item.hash_key.key_boxer_for(&parse_quote!(self.sk.unwrap()));

        quote! {
            struct #hash_key_struct<'a> {
                q: #get_item_struct<'a>,
            }

            struct #get_item_struct<'a> {
                table: &'a #table_struct,
                hk: Option<#hash_key_typ>,
                sk: Option<#sort_key_typ>,
            }

            impl<'a> #get_item_struct<'a> {
                fn new(table: &'a #table_struct) -> #hash_key_struct<'a> {
                    let q = #get_item_struct { table, hk: None, sk: None };
                    #hash_key_struct { q }
                }
            }

            impl<'a> #hash_key_struct<'a> {
                fn #hash_key_ident (mut self, v: impl Into<#hash_key_typ>) -> #sort_key_struct<'a> {
                    self.q.hk = Some(v.into());
                    #sort_key_struct { q: self.q }
                }
            }

            struct #sort_key_struct<'a> {
                q: #get_item_struct<'a>,
            }

            impl<'a> #sort_key_struct<'a> {
                fn #sort_key_ident (mut self, sk: impl Into<#sort_key_typ>) -> #get_item_struct<'a> {
                    self.q.sk = Some(sk.into());
                    self.q
                }
            }

            impl<'a> Into<::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>> for #get_item_struct<'a> {
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
            struct #hash_key_struct<'a> {
                q: #get_item_struct<'a>,
            }

            struct #get_item_struct<'a> {
                table: &'a #table_struct,
                hk: Option<#hash_key_typ>,
            }

            impl<'a> #get_item_struct<'a> {
                fn new(table: &'a #table_struct) -> #hash_key_struct<'a> {
                    let q = #get_item_struct { table, hk: None };
                    #hash_key_struct { q }
                }
            }

            impl<'a> #hash_key_struct<'a> {
                fn #hash_key_ident (mut self, v: impl Into<#hash_key_typ>) -> #get_item_struct<'a> {
                    self.q.hk = Some(v.into());
                    self.q
                }
            }

            impl<'a> Into<::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>> for #get_item_struct<'a> {
                fn into(self) -> ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue> {
                    let mut key_values = ::std::collections::HashMap::new();
                    key_values.insert(#hash_key_attr_name.to_string(), #hash_key_boxer);
                    key_values
                }
            }
        }
    };

    quote! {
        #builders

        impl<'a> #get_item_struct<'a> {
            async fn send(self) -> Result<
                Option<#item_struct>,
                ::aymond::shim::aws_sdk_dynamodb::error::SdkError<
                    ::aymond::shim::aws_sdk_dynamodb::operation::get_item::GetItemError,
                    ::aymond::shim::aws_sdk_dynamodb::config::http::HttpResponse,
                >,
            > {
                let res = self.raw(|r| r).await?;
                Ok(res.item().map(|e| e.into()))
            }

            async fn raw<F>(
                self,
                f: F,
            ) -> Result<
                ::aymond::shim::aws_sdk_dynamodb::operation::get_item::GetItemOutput,
                ::aymond::shim::aws_sdk_dynamodb::error::SdkError<
                    ::aymond::shim::aws_sdk_dynamodb::operation::get_item::GetItemError,
                    ::aymond::shim::aws_sdk_dynamodb::config::http::HttpResponse,
                >,
            >
            where
                F: FnOnce(#aws_sdk_dynamodb::operation::get_item::builders::GetItemFluentBuilder)
                -> #aws_sdk_dynamodb::operation::get_item::builders::GetItemFluentBuilder
            {
                f(self.table.client.get_item())
                    .table_name(&self.table.table_name)
                    .set_key(Some(self.into()))
                    .send()
                    .await
            }
        }
    }
}
