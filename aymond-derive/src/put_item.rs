use crate::definition::ItemDefinition;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, parse_quote};

pub fn create_put_item_builder(item: &ItemDefinition) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let item_struct = format_ident!("{}", &item.name);
    let table_struct = format_ident!("{}Table", &item.name);
    let put_item_struct = format_ident!("{}PutItem", &item.name);
    let condition_builder_struct = format_ident!("{}Condition", &item.name);

    let set_version_in_item = if let Some(ver_attr) = &item.version_attribute {
        let ver_field = &ver_attr.field;
        quote! { self.cond.set_version_value(v.#ver_field); }
    } else {
        quote! {}
    };

    let increment_version = if let Some(ver_attr) = &item.version_attribute {
        let ver_field = &ver_attr.field;
        quote! {
            if self.cond.is_versioning_enabled() {
                item.#ver_field += 1;
            }
        }
    } else {
        quote! {}
    };

    quote! {
        pub struct #put_item_struct<'a> {
            table: &'a #table_struct,
            i: Option<#item_struct>,
            cond: #condition_builder_struct,
        }

        impl<'a> #put_item_struct<'a> {
            fn new(table: &'a #table_struct) -> #put_item_struct<'a> {
                #put_item_struct { table, i: None, cond: #condition_builder_struct::new() }
            }
        }

        impl<'a> #put_item_struct<'a> {
            pub fn item(mut self, v: #item_struct) -> #put_item_struct<'a> {
                #set_version_in_item
                self.i = Some(v);
                self
            }

            pub fn condition<F, R>(mut self, f: F) -> #put_item_struct<'a>
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

            pub async fn raw<F>(
                self,
                f: F,
            ) -> Result<
                ::aymond::shim::aws_sdk_dynamodb::operation::put_item::PutItemOutput,
                ::aymond::shim::aws_sdk_dynamodb::error::SdkError<
                    ::aymond::shim::aws_sdk_dynamodb::operation::put_item::PutItemError,
                    ::aymond::shim::aws_sdk_dynamodb::config::http::HttpResponse,
                >,
            >
            where
                F: FnOnce(#aws_sdk_dynamodb::operation::put_item::builders::PutItemFluentBuilder)
                -> #aws_sdk_dynamodb::operation::put_item::builders::PutItemFluentBuilder
            {
                let mut item = self.i.expect("item not set");
                #increment_version
                let (cond_expr, expr_name, expr_value) = self.cond.build();
                let mut req = f(self.table.aymond.client.put_item())
                    .table_name(&self.table.table_name)
                    .set_item(Some(item.into()));
                if cond_expr.is_some() {
                    req = req.set_condition_expression(cond_expr)
                        .set_expression_attribute_names(expr_name)
                        .set_expression_attribute_values(expr_value);
                }
                req
                    .send()
                    .await
            }

            pub async fn send(self) -> Result<
                (),
                #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::put_item::PutItemError,
                    #aws_sdk_dynamodb::config::http::HttpResponse
                >
            > {
                self.raw(|r| r).await?;
                Ok(())
            }
        }

        impl<'a> Into<#aws_sdk_dynamodb::types::Put> for #put_item_struct<'a> {
            fn into(self) -> #aws_sdk_dynamodb::types::Put {
                let mut item = self.i.expect("item not set");
                #increment_version
                let (cond_expr, expr_name, expr_value) = self.cond.build();
                let mut b = #aws_sdk_dynamodb::types::Put::builder()
                    .table_name(&self.table.table_name)
                    .set_item(Some(item.into()));
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
