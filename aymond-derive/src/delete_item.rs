use crate::definition::ItemDefinition;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, parse_quote};

pub fn create_delete_builder(item: &ItemDefinition) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let table_struct = format_ident!("{}Table", &item.name);
    let delete_item_struct = format_ident!("{}DeleteItem", &item.name);
    let hash_key_struct = format_ident!("{}DeleteItemHashKey", &item.name);
    let condition_builder_struct = format_ident!("{}Condition", &item.name);

    let hash_key = item.hash_key.as_ref().unwrap();
    let hash_key_attr_name = &hash_key.ddb_name;
    let hash_key_ident = &hash_key.field;
    let hash_key_typ = &hash_key.ty;
    let hash_key_boxer = &hash_key.to_attribute_value(&parse_quote!(hk));

    let item_struct = format_ident!("{}", &item.name);

    let set_version_in_item = if let Some(ver_attr) = &item.version_attribute {
        let ver_field = &ver_attr.field;
        quote! { self.q.cond.set_version_value(v.#ver_field); }
    } else {
        quote! {}
    };

    let (builders, build_key_map) = if item.sort_key.is_some() {
        let sort_key_struct = format_ident!("{}DeleteItemSortKey", &item.name);
        let sort_key = item.sort_key.as_ref().unwrap();
        let sort_key_ident = &sort_key.field;
        let sort_key_attr_name = &sort_key.ddb_name;
        let sort_key_typ = &sort_key.ty;
        let sort_key_boxer = &sort_key.to_attribute_value(&parse_quote!(sk));

        let build_key_map = quote! {
            let hk = self.hk.unwrap();
            let mut key_values = ::std::collections::HashMap::new();
            key_values.insert(#hash_key_attr_name.to_string(), #hash_key_boxer);
            if self.sk.is_some() {
                let sk = self.sk.unwrap();
                key_values.insert(#sort_key_attr_name.to_string(), #sort_key_boxer);
            }
        };

        let builders = quote! {
            struct #hash_key_struct<'a> {
                q: #delete_item_struct<'a>,
            }

            struct #delete_item_struct<'a> {
                table: &'a #table_struct,
                hk: Option<#hash_key_typ>,
                sk: Option<#sort_key_typ>,
                cond: #condition_builder_struct,
            }

            impl<'a> #delete_item_struct<'a> {
                fn new(table: &'a #table_struct) -> #hash_key_struct<'a> {
                    let q = #delete_item_struct { table, hk: None, sk: None, cond: #condition_builder_struct::new() };
                    #hash_key_struct { q }
                }
            }

            impl<'a> #hash_key_struct<'a> {
                fn #hash_key_ident (mut self, v: impl Into<#hash_key_typ>) -> #sort_key_struct<'a> {
                    self.q.hk = Some(v.into());
                    #sort_key_struct { q: self.q }
                }

                fn item(mut self, v: #item_struct) -> #delete_item_struct<'a> {
                    #set_version_in_item
                    self.q.hk = Some(v.#hash_key_ident.into());
                    self.q.sk = Some(v.#sort_key_ident.into());
                    self.q
                }
            }

            struct #sort_key_struct<'a> {
                q: #delete_item_struct<'a>,
            }

            impl<'a> #sort_key_struct<'a> {
                fn #sort_key_ident (mut self, sk: impl Into<#sort_key_typ>) -> #delete_item_struct<'a> {
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
            struct #hash_key_struct<'a> {
                q: #delete_item_struct<'a>,
            }

            struct #delete_item_struct<'a> {
                table: &'a #table_struct,
                hk: Option<#hash_key_typ>,
                cond: #condition_builder_struct,
            }

            impl<'a> #delete_item_struct<'a> {
                fn new(table: &'a #table_struct) -> #hash_key_struct<'a> {
                    let q = #delete_item_struct { table, hk: None, cond: #condition_builder_struct::new() };
                    #hash_key_struct { q }
                }
            }

            impl<'a> #hash_key_struct<'a> {
                fn #hash_key_ident (mut self, v: impl Into<#hash_key_typ>) -> #delete_item_struct<'a> {
                    self.q.hk = Some(v.into());
                    self.q
                }

                fn item(mut self, v: #item_struct) -> #delete_item_struct<'a> {
                    #set_version_in_item
                    self.q.hk = Some(v.#hash_key_ident.into());
                    self.q
                }
            }
        };

        (builders, build_key_map)
    };

    quote! {
        #builders

        impl<'a> #delete_item_struct<'a> {
            fn condition<F>(mut self, f: F) -> #delete_item_struct<'a>
            where
                F: FnOnce(&mut #condition_builder_struct) -> ::aymond::condition::CondExpr
            {
                let expr = f(&mut self.cond);
                self.cond.set_expr(expr);
                self
            }

            async fn send(self) -> Result<
                (),
                #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::delete_item::DeleteItemError,
                    #aws_sdk_dynamodb::config::http::HttpResponse
                >
            > {
                self.raw(|r| r).await?;
                Ok(())
            }

            async fn raw<F>(
                self,
                f: F,
            ) -> Result<
                ::aymond::shim::aws_sdk_dynamodb::operation::delete_item::DeleteItemOutput,
                ::aymond::shim::aws_sdk_dynamodb::error::SdkError<
                    ::aymond::shim::aws_sdk_dynamodb::operation::delete_item::DeleteItemError,
                    ::aymond::shim::aws_sdk_dynamodb::config::http::HttpResponse,
                >,
            >
            where
                F: FnOnce(#aws_sdk_dynamodb::operation::delete_item::builders::DeleteItemFluentBuilder)
                -> #aws_sdk_dynamodb::operation::delete_item::builders::DeleteItemFluentBuilder
            {
                let (cond_expr, expr_name, expr_value) = self.cond.build();
                let table_name = &self.table.table_name;
                let client = &self.table.client;
                #build_key_map
                let mut req = f(client.delete_item())
                    .table_name(table_name)
                    .set_key(Some(key_values));
                if cond_expr.is_some() {
                    req = req.set_condition_expression(cond_expr)
                        .set_expression_attribute_names(expr_name)
                        .set_expression_attribute_values(expr_value);
                }
                req
                    .send()
                    .await
            }
        }

        impl<'a> Into<#aws_sdk_dynamodb::types::Delete> for #delete_item_struct<'a> {
            fn into(self) -> #aws_sdk_dynamodb::types::Delete {
                let (cond_expr, expr_name, expr_value) = self.cond.build();
                let table_name = &self.table.table_name;
                #build_key_map
                let mut b = #aws_sdk_dynamodb::types::Delete::builder()
                    .table_name(table_name)
                    .set_key(Some(key_values));
                if cond_expr.is_some() {
                    b = b.set_condition_expression(cond_expr)
                        .set_expression_attribute_names(expr_name)
                        .set_expression_attribute_values(expr_value);
                }
                b.build().unwrap()
            }
        }
    }
}
