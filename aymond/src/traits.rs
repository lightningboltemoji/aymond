use aws_sdk_dynamodb::{
    config::http::HttpResponse,
    error::SdkError,
    operation::{
        create_table::CreateTableError,
        delete_table::DeleteTableError,
        get_item::{GetItemError, GetItemOutput, builders::GetItemFluentBuilder},
        put_item::{PutItemError, PutItemOutput, builders::PutItemFluentBuilder},
        query::{QueryError, QueryOutput, builders::QueryFluentBuilder},
    },
    types::{AttributeDefinition, AttributeValue, KeySchemaElement},
};
use futures::Stream;
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

pub trait Table<T, Q, QHK>
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

    fn delete(
        &self,
        err_if_not_exists: bool,
    ) -> impl Future<Output = Result<(), SdkError<DeleteTableError, HttpResponse>>> + Send;

    fn get_item<F>(
        &self,
        key: HashMap<String, AttributeValue>,
        f: F,
    ) -> impl Future<Output = Result<GetItemOutput, SdkError<GetItemError, HttpResponse>>>
    where
        F: FnOnce(GetItemFluentBuilder) -> GetItemFluentBuilder;

    fn get(
        &self,
        key: HashMap<String, AttributeValue>,
    ) -> impl Future<Output = Result<Option<T>, SdkError<GetItemError, HttpResponse>>> + Send;

    fn put_item<F>(
        &self,
        t: T,
        f: F,
    ) -> impl Future<Output = Result<PutItemOutput, SdkError<PutItemError, HttpResponse>>>
    where
        F: FnOnce(PutItemFluentBuilder) -> PutItemFluentBuilder;

    fn put(
        &self,
        t: T,
    ) -> impl Future<Output = Result<(), SdkError<PutItemError, HttpResponse>>> + Send;

    fn query_ext<QF, F>(
        &self,
        q: QF,
        f: F,
    ) -> impl Future<Output = Result<QueryOutput, SdkError<QueryError, HttpResponse>>>
    where
        QF: FnOnce(QHK) -> Q,
        F: FnOnce(QueryFluentBuilder) -> QueryFluentBuilder;

    fn query<'a, QF>(
        &self,
        q: QF,
    ) -> impl Stream<Item = Result<T, SdkError<QueryError, HttpResponse>>> + 'a
    where
        QF: FnOnce(QHK) -> Q;
}
