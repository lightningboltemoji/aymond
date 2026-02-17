use std::sync::Arc;

use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_dynamodb::types::{Put, TransactWriteItem};
use aws_types::sdk_config::SharedCredentialsProvider;

pub mod prelude {
    pub use crate::traits::{Item, NestedItem, Table};
    pub use aymond_derive::aymond;
}

pub mod shim;
pub mod traits;

pub struct Tx<'a> {
    client: &'a HighLevelClient,
    put: Vec<Put>,
}

pub struct HighLevelClient {
    pub client: Arc<aws_sdk_dynamodb::Client>,
}

impl<'a> HighLevelClient {
    pub fn new_with_local_config(
        endpoint_url: impl Into<String>,
        region_name: impl Into<String>,
    ) -> Self {
        let credentials = Credentials::from_keys("empty", "empty", None);
        let endpoint_url = endpoint_url.into();
        let region_name = region_name.into();
        Self::new_with_config_builder(move |b| {
            b.credentials_provider(SharedCredentialsProvider::new(credentials))
                .region(Region::new(region_name))
                .endpoint_url(endpoint_url)
                .behavior_version(aws_sdk_dynamodb::config::BehaviorVersion::latest())
        })
    }

    pub fn new_with_config_builder<F>(builder: F) -> Self
    where
        F: FnOnce(aws_types::sdk_config::Builder) -> aws_types::sdk_config::Builder,
    {
        let config = builder(aws_types::SdkConfig::builder()).build();
        Self::new_with_config(config)
    }

    pub async fn new_with_default_config() -> Self {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        Self::new_with_config(config)
    }

    pub fn new_with_config(config: aws_types::SdkConfig) -> Self {
        let client = ::std::sync::Arc::new(aws_sdk_dynamodb::Client::new(&config));
        Self::new_with_client(client)
    }

    pub fn new_with_client(client: ::std::sync::Arc<aws_sdk_dynamodb::Client>) -> Self {
        Self { client }
    }

    pub fn tx(&'a self) -> Tx<'a> {
        Tx {
            client: self,
            put: vec![],
        }
    }
}

impl<'a> Tx<'a> {
    pub fn put(mut self, put: impl Into<Put>) -> Self {
        self.put.push(put.into());
        self
    }

    pub async fn raw<F>(
        self,
        f: F,
    ) -> Result<
        aws_sdk_dynamodb::operation::transact_write_items::TransactWriteItemsOutput,
        aws_sdk_dynamodb::error::SdkError<
            aws_sdk_dynamodb::operation::transact_write_items::TransactWriteItemsError,
            aws_sdk_dynamodb::config::http::HttpResponse
        >
    >
    where
        F: FnOnce(aws_sdk_dynamodb::operation::transact_write_items::builders::TransactWriteItemsFluentBuilder)
        -> aws_sdk_dynamodb::operation::transact_write_items::builders::TransactWriteItemsFluentBuilder
    {
        f(self.client.client.transact_write_items())
            .set_transact_items(self.into())
            .send()
            .await
    }

    pub async fn send(
        self,
    ) -> Result<
        (),
        aws_sdk_dynamodb::error::SdkError<
            aws_sdk_dynamodb::operation::transact_write_items::TransactWriteItemsError,
            aws_sdk_dynamodb::config::http::HttpResponse,
        >,
    > {
        self.raw(|r| r).await?;
        Ok(())
    }
}

impl<'a> Into<Option<Vec<TransactWriteItem>>> for Tx<'a> {
    fn into(self) -> Option<Vec<TransactWriteItem>> {
        let mut vec = vec![];
        for p in self.put {
            vec.push(TransactWriteItem::builder().put(p).build());
        }
        Some(vec)
    }
}
