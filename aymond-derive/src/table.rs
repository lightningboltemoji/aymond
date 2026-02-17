use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, parse_quote};

use crate::{
    ItemDefinition, create_query_builder, get_item::create_get_builder,
    put_item::create_put_item_builder,
};

pub fn create_table(def: &ItemDefinition) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let name = format_ident!("{}", &def.name);
    let table_struct = format_ident!("{}Table", &name);
    let get_item_struct = format_ident!("{}GetItem", &name);
    let get_item_hash_key_struct = format_ident!("{}GetItemHashKey", &name);
    let put_item_struct = format_ident!("{}PutItem", &name);
    let query_struct = format_ident!("{}Query", &name);
    let query_hash_key_struct = format_ident!("{}QueryHashKey", &name);

    let get_item = create_get_builder(def);
    let put_item = create_put_item_builder(def);
    let query = create_query_builder(def);
    quote! {
        #get_item
        #put_item
        #query

        #[derive(Debug)]
        struct #table_struct {
            client: ::std::sync::Arc<#aws_sdk_dynamodb::Client>,
            table_name: String,
        }

        impl<'a> Table<'a, #name, #get_item_struct<'a>, #get_item_hash_key_struct<'a>, #put_item_struct<'a>, #query_struct<'a>, #query_hash_key_struct<'a>> for #table_struct {

            fn new(
                client: &'a ::aymond::HighLevelClient,
                table_name: impl ::core::convert::Into<String>,
            ) -> Self {
                Self {
                    table_name: table_name.into(),
                    client: client.client.clone(),
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

            fn put(&'a self) -> #put_item_struct<'a> {
                #put_item_struct::new(self)
            }

            fn query(&'a self) -> #query_hash_key_struct<'a> {
                #query_struct::new(self)
            }
        }
    }
}
