use crate::definition::ItemDefinition;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, parse_quote};

pub fn create_batch_write_builder(item: &ItemDefinition) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let item_struct = format_ident!("{}", &item.name);
    let table_struct = format_ident!("{}Table", &item.name);
    let batch_write_struct = format_ident!("{}BatchWriteItem", &item.name);
    let batch_write_ops_struct = format_ident!("{}BatchWriteItemOps", &item.name);
    let delete_hash_key_struct = format_ident!("{}BatchWriteItemDeleteHashKey", &item.name);

    let hash_key = item.hash_key.as_ref().unwrap();
    let hash_key_attr_name = &hash_key.ddb_name;
    let hash_key_ident = &hash_key.field;
    let hash_key_typ = &hash_key.ty;
    let hash_key_boxer = hash_key.to_attribute_value(&parse_quote!(hk_val));

    let (delete_builders, initial_delete, ops_delete) = if let Some(sort_key) = &item.sort_key {
        let delete_sort_key_struct = format_ident!("{}BatchWriteItemDeleteSortKey", &item.name);
        let sort_key_attr_name = &sort_key.ddb_name;
        let sort_key_ident = &sort_key.field;
        let sort_key_typ = &sort_key.ty;
        let sort_key_boxer = sort_key.to_attribute_value(&parse_quote!(sk_val));

        let builders = quote! {
            pub struct #delete_hash_key_struct<'a> {
                table: &'a #table_struct,
                ops: Vec<#aws_sdk_dynamodb::types::WriteRequest>,
            }

            impl<'a> #delete_hash_key_struct<'a> {
                pub fn #hash_key_ident(self, v: impl Into<#hash_key_typ>) -> #delete_sort_key_struct<'a> {
                    #delete_sort_key_struct {
                        table: self.table,
                        ops: self.ops,
                        hk: v.into(),
                    }
                }
            }

            pub struct #delete_sort_key_struct<'a> {
                table: &'a #table_struct,
                ops: Vec<#aws_sdk_dynamodb::types::WriteRequest>,
                hk: #hash_key_typ,
            }

            impl<'a> #delete_sort_key_struct<'a> {
                pub fn #sort_key_ident(mut self, v: impl Into<#sort_key_typ>) -> #batch_write_ops_struct<'a> {
                    let hk_val = self.hk;
                    let sk_val: #sort_key_typ = v.into();
                    let mut key = ::std::collections::HashMap::new();
                    key.insert(#hash_key_attr_name.to_string(), #hash_key_boxer);
                    key.insert(#sort_key_attr_name.to_string(), #sort_key_boxer);
                    self.ops.push(
                        #aws_sdk_dynamodb::types::WriteRequest::builder()
                            .delete_request(
                                #aws_sdk_dynamodb::types::DeleteRequest::builder()
                                    .set_key(Some(key))
                                    .build()
                                    .unwrap()
                            )
                            .build()
                    );
                    #batch_write_ops_struct {
                        table: self.table,
                        ops: self.ops,
                    }
                }
            }
        };

        let initial_delete = quote! {
            pub fn delete(self) -> #delete_hash_key_struct<'a> {
                #delete_hash_key_struct {
                    table: self.table,
                    ops: Vec::new(),
                }
            }
        };

        let ops_delete = quote! {
            pub fn delete(self) -> #delete_hash_key_struct<'a> {
                #delete_hash_key_struct {
                    table: self.table,
                    ops: self.ops,
                }
            }
        };

        (builders, initial_delete, ops_delete)
    } else {
        let builders = quote! {
            pub struct #delete_hash_key_struct<'a> {
                table: &'a #table_struct,
                ops: Vec<#aws_sdk_dynamodb::types::WriteRequest>,
            }

            impl<'a> #delete_hash_key_struct<'a> {
                pub fn #hash_key_ident(mut self, v: impl Into<#hash_key_typ>) -> #batch_write_ops_struct<'a> {
                    let hk_val: #hash_key_typ = v.into();
                    let mut key = ::std::collections::HashMap::new();
                    key.insert(#hash_key_attr_name.to_string(), #hash_key_boxer);
                    self.ops.push(
                        #aws_sdk_dynamodb::types::WriteRequest::builder()
                            .delete_request(
                                #aws_sdk_dynamodb::types::DeleteRequest::builder()
                                    .set_key(Some(key))
                                    .build()
                                    .unwrap()
                            )
                            .build()
                    );
                    #batch_write_ops_struct {
                        table: self.table,
                        ops: self.ops,
                    }
                }
            }
        };

        let initial_delete = quote! {
            pub fn delete(self) -> #delete_hash_key_struct<'a> {
                #delete_hash_key_struct {
                    table: self.table,
                    ops: Vec::new(),
                }
            }
        };

        let ops_delete = quote! {
            pub fn delete(self) -> #delete_hash_key_struct<'a> {
                #delete_hash_key_struct {
                    table: self.table,
                    ops: self.ops,
                }
            }
        };

        (builders, initial_delete, ops_delete)
    };

    quote! {
        #delete_builders

        pub struct #batch_write_struct<'a> {
            table: &'a #table_struct,
        }

        impl<'a> #batch_write_struct<'a> {
            fn new(table: &'a #table_struct) -> Self {
                Self { table }
            }

            pub fn put(self, item: #item_struct) -> #batch_write_ops_struct<'a> {
                let wr = #aws_sdk_dynamodb::types::WriteRequest::builder()
                    .put_request(
                        #aws_sdk_dynamodb::types::PutRequest::builder()
                            .set_item(Some(item.into()))
                            .build()
                            .unwrap()
                    )
                    .build();
                #batch_write_ops_struct {
                    table: self.table,
                    ops: vec![wr],
                }
            }

            #initial_delete
        }

        pub struct #batch_write_ops_struct<'a> {
            table: &'a #table_struct,
            ops: Vec<#aws_sdk_dynamodb::types::WriteRequest>,
        }

        impl<'a> #batch_write_ops_struct<'a> {
            pub fn put(mut self, item: #item_struct) -> Self {
                let wr = #aws_sdk_dynamodb::types::WriteRequest::builder()
                    .put_request(
                        #aws_sdk_dynamodb::types::PutRequest::builder()
                            .set_item(Some(item.into()))
                            .build()
                            .unwrap()
                    )
                    .build();
                self.ops.push(wr);
                self
            }

            #ops_delete

            pub async fn send(self) -> Result<
                (),
                #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::batch_write_item::BatchWriteItemError,
                    #aws_sdk_dynamodb::config::http::HttpResponse,
                >,
            > {
                for chunk in self.ops.chunks(25) {
                    let mut pending: Vec<#aws_sdk_dynamodb::types::WriteRequest> = chunk.to_vec();
                    let mut retries: u32 = 0;

                    loop {
                        let request_items = pending;
                        let res = self.table.aymond.client.batch_write_item()
                            .request_items(&self.table.table_name, request_items)
                            .send()
                            .await?;

                        let has_unprocessed = res.unprocessed_items()
                            .and_then(|u| u.get(&self.table.table_name))
                            .and_then(|items| {
                                if items.is_empty() { None } else { Some(items.to_vec()) }
                            });

                        match has_unprocessed {
                            Some(unprocessed) => {
                                match (self.table.aymond.retry_strategy)(retries) {
                                    Some(duration) => {
                                        pending = unprocessed;
                                        retries += 1;
                                        ::aymond::shim::tokio::time::sleep(duration).await;
                                    }
                                    None => panic!("batch_write_item: unprocessed items remain after max retries"),
                                }
                            }
                            None => {
                                break;
                            }
                        }
                    }
                }

                Ok(())
            }

            pub async fn raw<F>(
                self,
                f: F,
            ) -> Result<
                #aws_sdk_dynamodb::operation::batch_write_item::BatchWriteItemOutput,
                #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::batch_write_item::BatchWriteItemError,
                    #aws_sdk_dynamodb::config::http::HttpResponse,
                >,
            >
            where
                F: FnOnce(#aws_sdk_dynamodb::operation::batch_write_item::builders::BatchWriteItemFluentBuilder)
                    -> #aws_sdk_dynamodb::operation::batch_write_item::builders::BatchWriteItemFluentBuilder
            {
                f(self.table.aymond.client.batch_write_item())
                    .request_items(&self.table.table_name, self.ops)
                    .send()
                    .await
            }
        }
    }
}
