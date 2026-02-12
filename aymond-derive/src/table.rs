use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, parse_quote};

use crate::{ItemDefinition, create_query_builder, get_builder::create_get_builder};

pub fn create_table(def: &ItemDefinition) -> TokenStream {
    let aws_types: Expr = parse_quote!(::aymond::shim::aws_types);
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let name = format_ident!("{}", &def.name);
    let table_struct = format_ident!("{}Table", &name);
    let get_item_struct = format_ident!("{}GetItem", &name);
    let get_item_hash_key_struct = format_ident!("{}GetItemHashKey", &name);
    let query_struct = format_ident!("{}Query", &name);
    let query_hash_key_struct = format_ident!("{}QueryHashKey", &name);

    let get_item = create_get_builder(&def);
    let query = create_query_builder(&def);
    quote! {
        #get_item
        #query

        #[derive(Debug)]
        struct #table_struct {
            client: ::std::sync::Arc<#aws_sdk_dynamodb::Client>,
            table_name: String,
        }

        impl<'a> Table<'a, #name, #get_item_struct<'a>, #get_item_hash_key_struct<'a>, #query_struct<'a>, #query_hash_key_struct<'a>> for #table_struct {

            fn new_with_local_config(
                table_name: impl Into<String>,
                endpoint_url: impl Into<String>,
                region_name: impl Into<String>,
            ) -> Self {
                let credentials = ::aymond::shim::aws_credential_types::Credentials::from_keys("empty", "empty", None);
                let table_name = table_name.into();
                let endpoint_url = endpoint_url.into();
                let region_name = region_name.into();
                Self::new_with_config_builder(table_name, move |b| {
                    b.credentials_provider(#aws_types::sdk_config::SharedCredentialsProvider::new(credentials))
                        .region(#aws_types::region::Region::new(region_name))
                        .endpoint_url(endpoint_url)
                        .behavior_version(#aws_sdk_dynamodb::config::BehaviorVersion::latest())
                })
            }

            fn new_with_config_builder<F>(table_name: impl ::core::convert::Into<String>, builder: F) -> Self
            where
                F: FnOnce(#aws_types::sdk_config::Builder) -> #aws_types::sdk_config::Builder {
                    let config = builder(#aws_types::SdkConfig::builder()).build();
                    Self::new_with_config(table_name, config)
                }

            async fn new_with_default_config(table_name: impl ::core::convert::Into<String>) -> Self {
                let config = ::aymond::shim::aws_config::load_defaults(
                    ::aymond::shim::aws_config::BehaviorVersion::latest()
                ).await;
                Self::new_with_config(table_name, config)
            }

            fn new_with_config(table_name: impl ::core::convert::Into<String>, config: #aws_types::SdkConfig) -> Self {
                let client = ::std::sync::Arc::new(#aws_sdk_dynamodb::Client::new(&config));
                Self::new_with_client(table_name, client)
            }

            fn new_with_client(
                table_name: impl ::core::convert::Into<String>,
                client: ::std::sync::Arc<#aws_sdk_dynamodb::Client>,
            ) -> Self {
                Self {
                    client,
                    table_name: table_name.into()
                }
            }

            async fn create(&self, err_if_exists: bool) -> Result<
                (), #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::create_table::CreateTableError,
                    #aws_sdk_dynamodb::config::http::HttpResponse
                >
            > {
                let res = self.client.create_table()
                    .table_name(&self.table_name)
                    .set_key_schema(Some(#name::key_schemas()))
                    .set_attribute_definitions(Some(#name::key_attribute_defintions()))
                    .billing_mode(#aws_sdk_dynamodb::types::BillingMode::PayPerRequest)
                    .send();
                match res.await {
                    Err(e) => match e {
                        #aws_sdk_dynamodb::error::SdkError::ServiceError(ref context)
                            if !err_if_exists && context.err().is_resource_in_use_exception() => Ok(()),
                        _ => Err(e)
                    }
                    _ => Ok(())
                }
            }

            async fn delete(&self, err_if_not_exists: bool) -> Result<
                (), #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::delete_table::DeleteTableError,
                    #aws_sdk_dynamodb::config::http::HttpResponse
                >
            > {
                let res = self.client.delete_table()
                    .table_name(&self.table_name)
                    .send();
                match res.await {
                    Err(e) => match e {
                        #aws_sdk_dynamodb::error::SdkError::ServiceError(ref context)
                            if !err_if_not_exists && context.err().is_resource_not_found_exception() => Ok(()),
                        _ => Err(e)
                    }
                    _ => Ok(())
                }
            }

            fn get(&'a self) -> #get_item_hash_key_struct<'a> {
                #get_item_struct::new(self)
            }

            async fn put_item<F>(&self, t: #name, f: F) -> Result<
                #aws_sdk_dynamodb::operation::put_item::PutItemOutput,
                #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::put_item::PutItemError,
                    #aws_sdk_dynamodb::config::http::HttpResponse
                >
            >
                where F: FnOnce(#aws_sdk_dynamodb::operation::put_item::builders::PutItemFluentBuilder)
                    -> #aws_sdk_dynamodb::operation::put_item::builders::PutItemFluentBuilder
            {
                f(self.client.put_item())
                    .table_name(&self.table_name)
                    .set_item(Some(t.into()))
                    .send()
                    .await
            }

            async fn put(&self, t: #name) -> Result<
                (),
                #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::put_item::PutItemError,
                    #aws_sdk_dynamodb::config::http::HttpResponse
                >
            > {
                self.put_item(t, |r| r).await?;
                Ok(())
            }

            fn query(&'a self) -> #query_hash_key_struct<'a> {
                #query_struct::new(self)
            }
        }
    }
}
