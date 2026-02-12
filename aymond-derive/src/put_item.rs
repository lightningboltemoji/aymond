use crate::definition::ItemDefinition;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, parse_quote};

pub fn create_put_item_builder(item: &ItemDefinition) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let item_struct = format_ident!("{}", &item.name);
    let table_struct = format_ident!("{}Table", &item.name);
    let put_item_struct = format_ident!("{}PutItem", &item.name);

    quote! {
        struct #put_item_struct<'a> {
            table: &'a #table_struct,
            i: Option<#item_struct>,
        }

        impl<'a> #put_item_struct<'a> {
            fn new(table: &'a #table_struct) -> #put_item_struct<'a> {
                #put_item_struct { table, i: None}
            }
        }

        impl<'a> #put_item_struct<'a> {
            fn item(mut self, v: #item_struct) -> #put_item_struct<'a> {
                self.i = Some(v);
                self
            }

            async fn raw<F>(
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
                f(self.table.client.put_item())
                    .table_name(&self.table.table_name)
                    .set_item(Some(self.i.expect("item not set").into()))
                    .send()
                    .await
            }

            async fn send(self) -> Result<
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
    }
}
