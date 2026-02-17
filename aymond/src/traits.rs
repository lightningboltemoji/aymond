use aws_sdk_dynamodb::{
    config::http::HttpResponse,
    error::SdkError,
    operation::{create_table::CreateTableError, delete_table::DeleteTableError},
    types::{AttributeDefinition, AttributeValue, KeySchemaElement},
};
use std::collections::HashMap;

use crate::HighLevelClient;

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

pub trait Table<'a, T, G, GHK, P, Q, QHK>
where
    T: Item,
    GHK: 'a,
{
    fn new(client: &'a HighLevelClient, table_name: impl Into<String>) -> Self;

    fn create(
        &self,
        err_if_exists: bool,
    ) -> impl Future<Output = Result<(), SdkError<CreateTableError, HttpResponse>>> + Send;

    fn delete(
        &self,
        err_if_not_exists: bool,
    ) -> impl Future<Output = Result<(), SdkError<DeleteTableError, HttpResponse>>> + Send;

    fn get(&'a self) -> GHK;

    fn put(&'a self) -> P;

    fn query(&'a self) -> QHK;
}
