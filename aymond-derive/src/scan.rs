use crate::definition::ItemDefinition;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, parse_quote};

pub fn create_scan_builder(item: &ItemDefinition) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let item_struct = format_ident!("{}", &item.name);
    let table_struct = format_ident!("{}Table", &item.name);
    let scan_struct = format_ident!("{}Scan", &item.name);

    quote! {
        pub struct #scan_struct<'a> {
            table: &'a #table_struct,
        }

        impl<'a> #scan_struct<'a> {
            fn new(table: &'a #table_struct) -> Self {
                Self { table }
            }

            pub async fn send(self) -> impl ::aymond::shim::futures::Stream<Item = Result<#item_struct, #aws_sdk_dynamodb::error::SdkError<
                #aws_sdk_dynamodb::operation::scan::ScanError,
                #aws_sdk_dynamodb::config::http::HttpResponse
            >>> + 'a {
                let pagination = self.table.client.scan()
                    .table_name(&self.table.table_name)
                    .into_paginator()
                    .items()
                    .send();
                ::aymond::shim::futures::stream::unfold(pagination, |mut p| async move {
                    p.next().await.map(|item| (item.map(|i| (&i).into()), p))
                })
            }

            pub async fn raw<F>(
                self,
                f: F,
            ) -> Result<
                #aws_sdk_dynamodb::operation::scan::ScanOutput,
                #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::scan::ScanError,
                    #aws_sdk_dynamodb::config::http::HttpResponse,
                >,
            >
            where
                F: FnOnce(#aws_sdk_dynamodb::operation::scan::builders::ScanFluentBuilder)
                    -> #aws_sdk_dynamodb::operation::scan::builders::ScanFluentBuilder,
            {
                f(self.table.client.scan())
                    .table_name(&self.table.table_name)
                    .send()
                    .await
            }
        }
    }
}
