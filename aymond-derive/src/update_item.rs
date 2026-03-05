use crate::definition::{ItemAttribute, ItemDefinition};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, parse_quote};

pub fn create_update_builder(item: &ItemDefinition) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let table_struct = format_ident!("{}Table", &item.name);
    let update_item_struct = format_ident!("{}UpdateItem", &item.name);
    let hash_key_struct = format_ident!("{}UpdateItemHashKey", &item.name);
    let expression_builder_struct = format_ident!("{}UpdateExpression", &item.name);
    let remove_fields_struct = format_ident!("{}UpdateRemoveFields", &item.name);
    let condition_builder_struct = format_ident!("{}Condition", &item.name);

    let hash_key = item.hash_key.as_ref().unwrap();
    let hash_key_attr_name = &hash_key.ddb_name;
    let hash_key_ident = &hash_key.field;
    let hash_key_typ = &hash_key.ty;
    let hash_key_boxer = &hash_key.to_attribute_value(&parse_quote!(hk));

    let updatable_attrs: Vec<&ItemAttribute> = item.other_attributes.iter().collect();
    let expression_accessors: Vec<TokenStream> = updatable_attrs
        .iter()
        .map(|attr| {
            let fn_name = &attr.field;
            let ddb_name = &attr.ddb_name;
            let return_type = attr.update_path_type();
            quote! {
                pub fn #fn_name(&self) -> #return_type {
                    ::aymond::update::UpdatePathRoot::with_prefix(
                        vec![::aymond::update::PathSegment::Attr(#ddb_name.to_string())]
                    )
                }
            }
        })
        .collect();

    let remove_accessors: Vec<TokenStream> = updatable_attrs
        .iter()
        .map(|attr| {
            let fn_name = &attr.field;
            let ddb_name = &attr.ddb_name;
            quote! {
                pub fn #fn_name(&self) -> ::aymond::update::UpdateExpr {
                    ::aymond::update::UpdateExpr::remove(
                        vec![::aymond::update::PathSegment::Attr(#ddb_name.to_string())]
                    )
                }
            }
        })
        .collect();

    let (builders, build_key_map) = if item.sort_key.is_some() {
        let sort_key_struct = format_ident!("{}UpdateItemSortKey", &item.name);
        let sort_key = item.sort_key.as_ref().unwrap();
        let sort_key_ident = &sort_key.field;
        let sort_key_attr_name = &sort_key.ddb_name;
        let sort_key_typ = &sort_key.ty;
        let sort_key_boxer = &sort_key.to_attribute_value(&parse_quote!(sk));

        let build_key_map = quote! {
            let hk = self.hk.unwrap();
            let sk = self.sk.unwrap();
            let mut key_values = ::std::collections::HashMap::new();
            key_values.insert(#hash_key_attr_name.to_string(), #hash_key_boxer);
            key_values.insert(#sort_key_attr_name.to_string(), #sort_key_boxer);
        };

        let builders = quote! {
            pub struct #hash_key_struct<'a> {
                q: #update_item_struct<'a>,
            }

            pub struct #update_item_struct<'a> {
                table: &'a #table_struct,
                hk: Option<#hash_key_typ>,
                sk: Option<#sort_key_typ>,
                cond: #condition_builder_struct,
                expr: Option<::aymond::update::UpdateExpr>,
            }

            impl<'a> #update_item_struct<'a> {
                fn new(table: &'a #table_struct) -> #hash_key_struct<'a> {
                    let q = #update_item_struct {
                        table,
                        hk: None,
                        sk: None,
                        cond: #condition_builder_struct::new(),
                        expr: None,
                    };
                    #hash_key_struct { q }
                }
            }

            impl<'a> #hash_key_struct<'a> {
                pub fn #hash_key_ident(mut self, v: impl Into<#hash_key_typ>) -> #sort_key_struct<'a> {
                    self.q.hk = Some(v.into());
                    #sort_key_struct { q: self.q }
                }
            }

            pub struct #sort_key_struct<'a> {
                q: #update_item_struct<'a>,
            }

            impl<'a> #sort_key_struct<'a> {
                pub fn #sort_key_ident(mut self, sk: impl Into<#sort_key_typ>) -> #update_item_struct<'a> {
                    self.q.sk = Some(sk.into());
                    self.q
                }
            }
        };
        (builders, build_key_map)
    } else {
        let build_key_map = quote! {
            let hk = self.hk.unwrap();
            let mut key_values = ::std::collections::HashMap::new();
            key_values.insert(#hash_key_attr_name.to_string(), #hash_key_boxer);
        };

        let builders = quote! {
            pub struct #hash_key_struct<'a> {
                q: #update_item_struct<'a>,
            }

            pub struct #update_item_struct<'a> {
                table: &'a #table_struct,
                hk: Option<#hash_key_typ>,
                cond: #condition_builder_struct,
                expr: Option<::aymond::update::UpdateExpr>,
            }

            impl<'a> #update_item_struct<'a> {
                fn new(table: &'a #table_struct) -> #hash_key_struct<'a> {
                    let q = #update_item_struct {
                        table,
                        hk: None,
                        cond: #condition_builder_struct::new(),
                        expr: None,
                    };
                    #hash_key_struct { q }
                }
            }

            impl<'a> #hash_key_struct<'a> {
                pub fn #hash_key_ident(mut self, v: impl Into<#hash_key_typ>) -> #update_item_struct<'a> {
                    self.q.hk = Some(v.into());
                    self.q
                }
            }
        };
        (builders, build_key_map)
    };

    quote! {
        pub struct #expression_builder_struct;
        pub struct #remove_fields_struct;

        impl #expression_builder_struct {
            fn new() -> Self {
                Self
            }

            pub fn remove(&self) -> #remove_fields_struct {
                #remove_fields_struct
            }

            #( #expression_accessors )*
        }

        impl #remove_fields_struct {
            #( #remove_accessors )*
        }

        #builders

        impl<'a> #update_item_struct<'a> {
            pub fn expression<F, R>(mut self, f: F) -> #update_item_struct<'a>
            where
                F: FnOnce(&#expression_builder_struct) -> R,
                R: ::aymond::update::IntoOptionalUpdateExpr,
            {
                let result = f(&#expression_builder_struct::new());
                if let Some(expr) = result.into_optional_update_expr() {
                    self.expr = Some(expr);
                }
                self
            }

            pub fn condition<F, R>(mut self, f: F) -> #update_item_struct<'a>
            where
                F: FnOnce(&#condition_builder_struct) -> R,
                R: ::aymond::condition::IntoOptionalCondExpr,
            {
                let result = f(&self.cond);
                if let Some(expr) = result.into_optional_cond_expr() {
                    self.cond.set_expr(expr);
                }
                self
            }

            pub async fn send(self) -> Result<
                (),
                #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::update_item::UpdateItemError,
                    #aws_sdk_dynamodb::config::http::HttpResponse
                >
            > {
                self.raw(|r| r).await?;
                Ok(())
            }

            pub async fn raw<F>(
                self,
                f: F,
            ) -> Result<
                #aws_sdk_dynamodb::operation::update_item::UpdateItemOutput,
                #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::update_item::UpdateItemError,
                    #aws_sdk_dynamodb::config::http::HttpResponse,
                >,
            >
            where
                F: FnOnce(#aws_sdk_dynamodb::operation::update_item::builders::UpdateItemFluentBuilder)
                -> #aws_sdk_dynamodb::operation::update_item::builders::UpdateItemFluentBuilder
            {
                let (cond_expr, cond_names, cond_values) = self.cond.build();
                let (update_expr, update_names, update_values) =
                    self.expr.expect("update expression not set").build();

                let mut names = ::std::collections::HashMap::new();
                if let Some(cond_names) = cond_names {
                    names.extend(cond_names);
                }
                names.extend(update_names);
                let names = if names.is_empty() { None } else { Some(names) };

                let mut values = ::std::collections::HashMap::new();
                if let Some(cond_values) = cond_values {
                    values.extend(cond_values);
                }
                values.extend(update_values);
                let values = if values.is_empty() { None } else { Some(values) };

                let table_name = &self.table.table_name;
                let client = &self.table.aymond.client;
                #build_key_map

                f(client.update_item())
                    .table_name(table_name)
                    .set_key(Some(key_values))
                    .set_update_expression(Some(update_expr))
                    .set_condition_expression(cond_expr)
                    .set_expression_attribute_names(names)
                    .set_expression_attribute_values(values)
                    .send()
                    .await
            }
        }

        impl<'a> Into<#aws_sdk_dynamodb::types::Update> for #update_item_struct<'a> {
            fn into(self) -> #aws_sdk_dynamodb::types::Update {
                let (cond_expr, cond_names, cond_values) = self.cond.build();
                let (update_expr, update_names, update_values) =
                    self.expr.expect("update expression not set").build();

                let mut names = ::std::collections::HashMap::new();
                if let Some(cond_names) = cond_names {
                    names.extend(cond_names);
                }
                names.extend(update_names);
                let names = if names.is_empty() { None } else { Some(names) };

                let mut values = ::std::collections::HashMap::new();
                if let Some(cond_values) = cond_values {
                    values.extend(cond_values);
                }
                values.extend(update_values);
                let values = if values.is_empty() { None } else { Some(values) };

                let table_name = &self.table.table_name;
                #build_key_map

                #aws_sdk_dynamodb::types::Update::builder()
                    .table_name(table_name)
                    .set_key(Some(key_values))
                    .set_update_expression(Some(update_expr))
                    .set_condition_expression(cond_expr)
                    .set_expression_attribute_names(names)
                    .set_expression_attribute_values(values)
                    .build()
                    .unwrap()
            }
        }
    }
}
