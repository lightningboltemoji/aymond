use aws_sdk_dynamodb::{
    config::http::HttpResponse,
    error::SdkError,
    operation::{
        create_table::CreateTableError,
        get_item::{GetItemError, GetItemOutput},
        put_item::{PutItemError, PutItemOutput},
    },
    types::{AttributeDefinition, AttributeValue, KeySchemaElement},
};
use std::{collections::HashMap, sync::Arc};

pub trait NestedItem:
    for<'a> From<&'a HashMap<String, AttributeValue>> + Into<HashMap<String, AttributeValue>>
{
}

pub trait Item:
    for<'a> From<&'a HashMap<String, AttributeValue>> + Into<HashMap<String, AttributeValue>>
{
    fn key_schemas() -> Vec<KeySchemaElement>;
    fn key_attribute_defintions() -> Vec<AttributeDefinition>;
}

pub trait Table<T>
where
    T: Item,
{
    fn new_with_default_config(table_name: impl Into<String>) -> impl Future<Output = Self>;

    fn new_with_local_config(
        table_name: impl Into<String>,
        endpoint_url: impl Into<String>,
        region_name: impl Into<String>,
    ) -> Self;

    fn new_with_config_builder<F>(table_name: impl Into<String>, builder: F) -> Self
    where
        F: FnOnce(::aws_types::sdk_config::Builder) -> ::aws_types::sdk_config::Builder;

    fn new_with_config(table_name: impl Into<String>, config: ::aws_types::SdkConfig) -> Self;

    fn new_with_client(
        table_name: impl Into<String>,
        client: Arc<aws_sdk_dynamodb::Client>,
    ) -> Self;

    fn create(
        &self,
        err_if_exists: bool,
    ) -> impl Future<Output = Result<(), SdkError<CreateTableError, HttpResponse>>> + Send;

    fn get(
        &self,
        key: HashMap<String, AttributeValue>,
    ) -> impl Future<Output = Result<GetItemOutput, SdkError<GetItemError, HttpResponse>>> + Send;

    fn put(
        &self,
        t: T,
    ) -> impl Future<Output = Result<PutItemOutput, SdkError<PutItemError, HttpResponse>>> + Send;
}
