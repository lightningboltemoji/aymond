use aws_sdk_dynamodb::{
    config::http::HttpResponse,
    error::SdkError,
    operation::create_table::{CreateTableError, CreateTableOutput},
    types::{AttributeDefinition, AttributeValue, KeySchemaElement},
};
use std::{collections::HashMap, sync::Arc};

pub trait Item:
    for<'a> From<&'a HashMap<String, AttributeValue>> + Into<HashMap<String, AttributeValue>>
{
    fn key_schemas() -> Vec<KeySchemaElement>;
    fn key_attribute_defintions() -> Vec<AttributeDefinition>;
}

pub trait Table {
    fn new(client: Arc<aws_sdk_dynamodb::Client>, table_name: impl Into<String>) -> Self;
    fn create_table(
        &self,
    ) -> impl Future<Output = Result<CreateTableOutput, SdkError<CreateTableError, HttpResponse>>> + Send;
}
