use crate::definition::ItemDefinition;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, parse_quote};

pub fn create_query_builder(item: &ItemDefinition) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let item_struct = format_ident!("{}", &item.name);
    let table_struct = format_ident!("{}Table", &item.name);
    let query_struct = format_ident!("{}Query", &item.name);
    let hash_key_struct = format_ident!("{}QueryHashKey", &item.name);

    let hash_key_attr_name = &item.hash_key.attr_name;
    let hash_key_ident = &item.hash_key.ident;
    let hash_key_typ = &item.hash_key.typ_ident;
    let hash_key_boxer = &item.hash_key.key_boxer_for(&parse_quote!(self.hk.unwrap()));

    let mut chunks: Vec<TokenStream> = vec![];

    if item.sort_key.is_some() {
        let sort_key_struct = format_ident!("{}QuerySortKey", &item.name);
        let sort_key_ident = &item.sort_key.as_ref().unwrap().ident;
        let sort_key_attr_name = &item.sort_key.as_ref().unwrap().attr_name;
        let sort_key_typ = &item.sort_key.as_ref().unwrap().typ_ident;

        let sort_key = item.sort_key.as_ref().unwrap();
        let sort_key_b_boxer = sort_key.key_boxer_for(&parse_quote!(self.b.unwrap()));
        let sort_key_c_boxer = sort_key.key_boxer_for(&parse_quote!(self.c.unwrap()));

        chunks.push(quote! {
            struct #hash_key_struct<'a> {
                q: #query_struct<'a>,
            }

            struct #query_struct<'a> {
                table: &'a #table_struct,
                hk: Option<#hash_key_typ>,
                qs: Option<String>,
                b: Option<#sort_key_typ>,
                c: Option<#sort_key_typ>,
            }

            impl<'a> #query_struct<'a> {
                fn new(table: &'a #table_struct) -> #hash_key_struct<'a> {
                    let q = #query_struct { table, hk: None, qs: None, b: None, c: None };
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
                q: #query_struct<'a>,
            }

            impl<'a> Into<(
                String,
                ::std::collections::HashMap<String, String>,
                ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>,
            )> for #query_struct<'a> {
                fn into(self) -> (
                    String,
                    ::std::collections::HashMap<String, String>,
                    ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>,
                ) {
                    let exp = self.qs.unwrap();

                    let mut key_names = ::std::collections::HashMap::new();
                    key_names.insert("#hk".to_string(), #hash_key_attr_name.to_string());
                    key_names.insert("#sk".to_string(), #sort_key_attr_name.to_string());

                    let mut key_values = ::std::collections::HashMap::new();
                    key_values.insert(":hk".to_string(), #hash_key_boxer);
                    if self.b.is_some() {
                        key_values.insert(":b".to_string(), #sort_key_b_boxer);
                    }
                    if self.c.is_some() {
                        key_values.insert(":c".to_string(), #sort_key_c_boxer);
                    }

                    (exp, key_names, key_values)
                }
            }
        });

        let mut comparisons = vec![
            (
                format_ident!("{}_gt", sort_key_ident),
                "#hk = :hk AND #sk > :b",
                vec![quote! {b}],
            ),
            (
                format_ident!("{}_ge", sort_key_ident),
                "#hk = :hk AND #sk >= :b",
                vec![quote! {b}],
            ),
            (
                format_ident!("{}_lt", sort_key_ident),
                "#hk = :hk AND #sk < :b",
                vec![quote! {b}],
            ),
            (
                format_ident!("{}_le", sort_key_ident),
                "#hk = :hk AND #sk <= :b",
                vec![quote! {b}],
            ),
            (
                format_ident!("{}_between", sort_key_ident),
                "#hk = :hk AND #sk BETWEEN :b AND :c",
                vec![quote! {b}, quote! {c}],
            ),
        ];
        if hash_key_typ == "String" {
            comparisons.push((
                format_ident!("{}_begins_with", sort_key_ident),
                "#hk = :hk AND begins_with(#sk, :b)",
                vec![quote! {b}],
            ));
        }

        for (fn_name, key_expression, vars) in comparisons {
            chunks.push(quote! {
                impl<'a> #sort_key_struct<'a> {
                    fn #fn_name (mut self, #( #vars: impl Into<#sort_key_typ>, )*) -> #query_struct<'a> {
                        self.q.qs = Some(#key_expression.into());
                        #( self.q.#vars = Some(#vars.into()); )*
                        self.q
                    }
                }
            })
        }
    } else {
        chunks.push(quote! {
            struct #hash_key_struct<'a> {
                q: #query_struct<'a>,
            }

            struct #query_struct<'a> {
                table: &'a #table_struct,
                hk: Option<#hash_key_typ>,
                qs: Option<String>,
            }

            impl<'a> #query_struct<'a> {
                fn new(table: &'a #table_struct) -> #hash_key_struct {
                    let q = #query_struct { table, hk: None, qs: None };
                    #hash_key_struct { q }
                }
            }

            impl<'a> #hash_key_struct<'a> {
                fn #hash_key_ident (mut self, v: impl Into<#hash_key_typ>) -> #query_struct<'a> {
                    self.q.hk = Some(v.into());
                    self.q.qs = Some("#hk = :hk".into());
                    self.q
                }
            }

            impl<'a> Into<(
                String,
                ::std::collections::HashMap<String, String>,
                ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>,
            )> for #query_struct<'a> {
                fn into(self) -> (
                    String,
                    ::std::collections::HashMap<String, String>,
                    ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>,
                ) {
                    let exp = self.qs.unwrap();

                    let mut key_names = ::std::collections::HashMap::new();
                    key_names.insert("#hk".to_string(), #hash_key_attr_name.to_string());

                    let mut key_values = ::std::collections::HashMap::new();
                    key_values.insert(":hk".to_string(), #hash_key_boxer);

                    (exp, key_names, key_values)
                }
            }
        });
    }

    quote! {
        #( #chunks )*

        impl<'a> #query_struct<'a> {
            async fn send(self) -> impl ::aymond::shim::futures::Stream<Item = Result<#item_struct, ::aymond::shim::aws_sdk_dynamodb::error::SdkError<
                ::aymond::shim::aws_sdk_dynamodb::operation::query::QueryError,
                ::aymond::shim::aws_sdk_dynamodb::config::http::HttpResponse
            >>> + 'a
            {
                let query = self.table.client.query();
                let table_name = &self.table.table_name;
                let (key_expr, attr_names, attr_values) = self.into();
                let pagination = query
                    .table_name(table_name)
                    .set_key_condition_expression(Some(key_expr))
                    .set_expression_attribute_names(Some(attr_names))
                    .set_expression_attribute_values(Some(attr_values))
                    .into_paginator()
                    .items()
                    .send();
                ::aymond::shim::futures::stream::unfold(pagination, |mut p| async move {
                    p.next().await.map(|item| (item.map(|i| (&i).into()), p))
                })
            }

            async fn raw<F>(
                self,
                f: F,
            ) -> Result<
                ::aymond::shim::aws_sdk_dynamodb::operation::query::QueryOutput,
                ::aymond::shim::aws_sdk_dynamodb::error::SdkError<
                    ::aymond::shim::aws_sdk_dynamodb::operation::query::QueryError,
                    ::aymond::shim::aws_sdk_dynamodb::config::http::HttpResponse,
                >,
            >
            where
                F: FnOnce(#aws_sdk_dynamodb::operation::query::builders::QueryFluentBuilder)
                -> #aws_sdk_dynamodb::operation::query::builders::QueryFluentBuilder
            {
                let query = f(self.table.client.query());
                let table_name = &self.table.table_name;
                let (key_expr, attr_names, attr_values) = self.into();
                query
                    .table_name(table_name)
                    .set_key_condition_expression(Some(key_expr))
                    .set_expression_attribute_names(Some(attr_names))
                    .set_expression_attribute_values(Some(attr_values))
                    .send()
                    .await
            }
        }
    }
}
