use crate::definition::ItemDefinition;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, parse_quote};

pub fn create_batch_get_builder(item: &ItemDefinition) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let item_struct = format_ident!("{}", &item.name);
    let table_struct = format_ident!("{}Table", &item.name);
    let batch_get_struct = format_ident!("{}BatchGetItem", &item.name);
    let batch_get_keys_struct = format_ident!("{}BatchGetItemKeys", &item.name);

    let hash_key = item.hash_key.as_ref().unwrap();
    let hash_key_attr_name = &hash_key.ddb_name;
    let hash_key_typ = &hash_key.ty;
    let hash_key_ident = &hash_key.field;

    let (_key_method_name, key_method_sig, key_body) = if let Some(sort_key) = &item.sort_key {
        let sort_key_attr_name = &sort_key.ddb_name;
        let sort_key_typ = &sort_key.ty;
        let sort_key_ident = &sort_key.field;

        let method_name = format_ident!("{}_and_{}", hash_key_ident, sort_key_ident);

        let hk_boxer = hash_key.to_attribute_value(&parse_quote!(hk_val));
        let sk_boxer = sort_key.to_attribute_value(&parse_quote!(sk_val));

        let sig: TokenStream = quote! {
            fn #method_name(mut self, hk: impl Into<#hash_key_typ>, sk: impl Into<#sort_key_typ>)
        };
        let body: TokenStream = quote! {
            let hk_val: #hash_key_typ = hk.into();
            let sk_val: #sort_key_typ = sk.into();
            let mut key = ::std::collections::HashMap::new();
            key.insert(#hash_key_attr_name.to_string(), #hk_boxer);
            key.insert(#sort_key_attr_name.to_string(), #sk_boxer);
            self.keys.push(key);
        };

        (method_name, sig, body)
    } else {
        let method_name = hash_key_ident.clone();

        let hk_boxer = hash_key.to_attribute_value(&parse_quote!(hk_val));

        let sig: TokenStream = quote! {
            fn #method_name(mut self, hk: impl Into<#hash_key_typ>)
        };
        let body: TokenStream = quote! {
            let hk_val: #hash_key_typ = hk.into();
            let mut key = ::std::collections::HashMap::new();
            key.insert(#hash_key_attr_name.to_string(), #hk_boxer);
            self.keys.push(key);
        };

        (method_name, sig, body)
    };

    quote! {
        pub struct #batch_get_struct<'a> {
            table: &'a #table_struct,
            keys: Vec<::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>>,
        }

        impl<'a> #batch_get_struct<'a> {
            fn new(table: &'a #table_struct) -> Self {
                Self { table, keys: Vec::new() }
            }

            pub #key_method_sig -> #batch_get_keys_struct<'a> {
                #key_body
                #batch_get_keys_struct {
                    table: self.table,
                    keys: self.keys,
                }
            }
        }

        pub struct #batch_get_keys_struct<'a> {
            table: &'a #table_struct,
            keys: Vec<::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>>,
        }

        impl<'a> #batch_get_keys_struct<'a> {
            pub #key_method_sig -> Self {
                #key_body
                self
            }

            pub async fn send(self) -> Result<
                Vec<#item_struct>,
                #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::batch_get_item::BatchGetItemError,
                    #aws_sdk_dynamodb::config::http::HttpResponse,
                >,
            > {
                let mut all_results: Vec<#item_struct> = Vec::new();

                for chunk in self.keys.chunks(100) {
                    let mut pending_keys: Vec<::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>>
                        = chunk.to_vec();
                    let mut retries: u32 = 0;

                    loop {
                        let request_keys = pending_keys;
                        let keys_and_attrs = #aws_sdk_dynamodb::types::KeysAndAttributes::builder()
                            .set_keys(Some(request_keys))
                            .build()
                            .unwrap();

                        let res = self.table.aymond.client.batch_get_item()
                            .request_items(&self.table.table_name, keys_and_attrs)
                            .send()
                            .await?;

                        if let Some(responses) = res.responses() {
                            if let Some(items) = responses.get(&self.table.table_name) {
                                for item in items {
                                    all_results.push(item.into());
                                }
                            }
                        }

                        let has_unprocessed = res.unprocessed_keys()
                            .and_then(|u| u.get(&self.table.table_name))
                            .and_then(|k| {
                                let keys = k.keys();
                                if keys.is_empty() { None } else { Some(keys.to_vec()) }
                            });

                        match has_unprocessed {
                            Some(unprocessed) => {
                                match (self.table.aymond.retry_strategy)(retries) {
                                    Some(duration) => {
                                        pending_keys = unprocessed;
                                        retries += 1;
                                        ::aymond::shim::tokio::time::sleep(duration).await;
                                    }
                                    None => panic!("batch_get_item: unprocessed keys remain after max retries"),
                                }
                            }
                            None => {
                                break;
                            }
                        }
                    }
                }

                Ok(all_results)
            }

            pub async fn raw<F>(
                self,
                f: F,
            ) -> Result<
                #aws_sdk_dynamodb::operation::batch_get_item::BatchGetItemOutput,
                #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::batch_get_item::BatchGetItemError,
                    #aws_sdk_dynamodb::config::http::HttpResponse,
                >,
            >
            where
                F: FnOnce(#aws_sdk_dynamodb::operation::batch_get_item::builders::BatchGetItemFluentBuilder)
                    -> #aws_sdk_dynamodb::operation::batch_get_item::builders::BatchGetItemFluentBuilder
            {
                let keys_and_attrs = #aws_sdk_dynamodb::types::KeysAndAttributes::builder()
                    .set_keys(Some(self.keys))
                    .build()
                    .unwrap();

                f(self.table.aymond.client.batch_get_item())
                    .request_items(&self.table.table_name, keys_and_attrs)
                    .send()
                    .await
            }
        }
    }
}
