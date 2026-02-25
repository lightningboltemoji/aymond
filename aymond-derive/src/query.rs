use crate::{
    definition::{ItemAttribute, ItemDefinition},
    util::{to_ident_format, to_pascal_case},
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, parse_quote};

pub fn create_main_query_builder(item: &ItemDefinition) -> TokenStream {
    let hash_key = item.hash_key.as_ref().unwrap();
    let sort_key = item.sort_key.as_ref();
    create_query_builder(&item.name, &item.name, hash_key, sort_key, None)
}

pub fn create_index_query_builders(item: &ItemDefinition) -> TokenStream {
    let table_struct = format_ident!("{}Table", &item.name);

    let mut chunks: Vec<TokenStream> = vec![];

    // GSIs sorted by name for deterministic output
    let mut gsis: Vec<_> = item.global_secondary_indexes.values().collect();
    gsis.sort_by_key(|g| &g.name);

    for gsi in &gsis {
        let gsi_hash_key = gsi.hash_key.as_ref().unwrap();
        let normalized = to_ident_format(&gsi.name);
        let pascal = to_pascal_case(&normalized);
        let prefix = format!("{}Index{}", item.name, pascal);
        let index_hk_struct = format_ident!("{}QueryHashKey", prefix);

        let builder = create_query_builder(
            &item.name,
            &prefix,
            gsi_hash_key,
            gsi.sort_key.as_ref(),
            Some(&gsi.name),
        );
        chunks.push(builder);

        let index_query_struct = format_ident!("{}Query", prefix);
        let method_name = format_ident!("query_{}", normalized);
        chunks.push(quote! {
            impl<'a> #table_struct {
                fn #method_name(&'a self) -> #index_hk_struct<'a> {
                    #index_query_struct::new(self)
                }
            }
        });
    }

    // LSIs sorted by name for deterministic output
    let mut lsis: Vec<_> = item.local_secondary_indexes.values().collect();
    lsis.sort_by_key(|l| &l.name);

    for lsi in &lsis {
        let table_hash_key = item.hash_key.as_ref().unwrap();
        let normalized = to_ident_format(&lsi.name);
        let pascal = to_pascal_case(&normalized);
        let prefix = format!("{}Index{}", item.name, pascal);
        let index_hk_struct = format_ident!("{}QueryHashKey", prefix);
        let index_query_struct = format_ident!("{}Query", prefix);

        let builder = create_query_builder(
            &item.name,
            &prefix,
            table_hash_key,
            Some(&lsi.sort_key),
            Some(&lsi.name),
        );
        chunks.push(builder);

        let method_name = format_ident!("query_{}", normalized);
        chunks.push(quote! {
            impl<'a> #table_struct {
                fn #method_name(&'a self) -> #index_hk_struct<'a> {
                    #index_query_struct::new(self)
                }
            }
        });
    }

    quote! { #( #chunks )* }
}

pub fn create_query_builder(
    item_name: &str,
    prefix: &str,
    hash_key: &ItemAttribute,
    sort_key: Option<&ItemAttribute>,
    index_name: Option<&str>,
) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let item_struct = format_ident!("{}", item_name);
    let table_struct = format_ident!("{}Table", item_name);
    let query_struct = format_ident!("{}Query", prefix);
    let hash_key_struct = format_ident!("{}QueryHashKey", prefix);

    let hash_key_attr_name = &hash_key.ddb_name;
    let hash_key_ident = &hash_key.field;
    let hash_key_typ = &hash_key.ty;
    let hash_key_boxer = hash_key.to_attribute_value(&parse_quote!(self.hk.unwrap()));

    let index_name_init = match index_name {
        Some(name) => quote! { Some(#name.to_string()) },
        None => quote! { None },
    };

    let mut chunks: Vec<TokenStream> = vec![];

    if let Some(sort_key_attr) = sort_key {
        let sort_key_struct = format_ident!("{}QuerySortKey", prefix);
        let sort_key_ident = &sort_key_attr.field;
        let sort_key_attr_name = &sort_key_attr.ddb_name;
        let sort_key_typ = &sort_key_attr.ty;

        let sort_key_b_boxer = sort_key_attr.to_attribute_value(&parse_quote!(self.b.unwrap()));
        let sort_key_c_boxer = sort_key_attr.to_attribute_value(&parse_quote!(self.c.unwrap()));

        chunks.push(quote! {
            struct #hash_key_struct<'a> {
                q: #query_struct<'a>,
            }

            struct #query_struct<'a> {
                table: &'a #table_struct,
                index_name: Option<String>,
                hk: Option<#hash_key_typ>,
                qs: Option<String>,
                b: Option<#sort_key_typ>,
                c: Option<#sort_key_typ>,
            }

            impl<'a> #query_struct<'a> {
                fn new(table: &'a #table_struct) -> #hash_key_struct<'a> {
                    let q = #query_struct { table, index_name: #index_name_init, hk: None, qs: None, b: None, c: None };
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
            ("eq", "#hk = :hk AND #sk = :b", vec![quote! {b}]),
            ("gt", "#hk = :hk AND #sk > :b", vec![quote! {b}]),
            ("ge", "#hk = :hk AND #sk >= :b", vec![quote! {b}]),
            ("lt", "#hk = :hk AND #sk < :b", vec![quote! {b}]),
            ("le", "#hk = :hk AND #sk <= :b", vec![quote! {b}]),
            (
                "between",
                "#hk = :hk AND #sk BETWEEN :b AND :c",
                vec![quote! {b}, quote! {c}],
            ),
        ];
        let sk_hier = &sort_key_attr.generics_hierarchy;
        let sk_supports_begins_with = match sk_hier.as_slice() {
            [t] => t == "String",
            [v, u] => v == "Vec" && u == "u8",
            _ => false,
        };
        if sk_supports_begins_with {
            comparisons.push((
                "begins_with",
                "#hk = :hk AND begins_with(#sk, :b)",
                vec![quote! {b}],
            ));
        }

        for (suffix, key_expression, vars) in comparisons {
            let fn_name = format_ident!("{}_{}", sort_key_ident, suffix);
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
                index_name: Option<String>,
                hk: Option<#hash_key_typ>,
                qs: Option<String>,
            }

            impl<'a> #query_struct<'a> {
                fn new(table: &'a #table_struct) -> #hash_key_struct {
                    let q = #query_struct { table, index_name: #index_name_init, hk: None, qs: None };
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
            async fn send(self) -> impl ::aymond::shim::futures::Stream<Item = Result<#item_struct, #aws_sdk_dynamodb::error::SdkError<
                #aws_sdk_dynamodb::operation::query::QueryError,
                #aws_sdk_dynamodb::config::http::HttpResponse
            >>> + 'a
            {
                let index_name = self.index_name.clone();
                let query = self.table.client.query();
                let table_name = &self.table.table_name;
                let (key_expr, attr_names, attr_values) = self.into();
                let pagination = query
                    .table_name(table_name)
                    .set_index_name(index_name.clone())
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
                #aws_sdk_dynamodb::operation::query::QueryOutput,
                #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::query::QueryError,
                    #aws_sdk_dynamodb::config::http::HttpResponse,
                >,
            >
            where
                F: FnOnce(#aws_sdk_dynamodb::operation::query::builders::QueryFluentBuilder)
                -> #aws_sdk_dynamodb::operation::query::builders::QueryFluentBuilder
            {
                let index_name = self.index_name.clone();
                let query = f(self.table.client.query());
                let table_name = &self.table.table_name;
                let (key_expr, attr_names, attr_values) = self.into();
                query
                    .table_name(table_name)
                    .set_index_name(index_name.clone())
                    .set_key_condition_expression(Some(key_expr))
                    .set_expression_attribute_names(Some(attr_names))
                    .set_expression_attribute_values(Some(attr_values))
                    .send()
                    .await
            }
        }
    }
}
