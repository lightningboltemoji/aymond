use std::sync::Arc;

use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_dynamodb::types::{ConditionCheck, Delete, Put, TransactWriteItem, Update};
use aws_types::sdk_config::SharedCredentialsProvider;

pub mod prelude {
    pub use crate::retry::{ExponentialBackoff, RetryStrategy};
    pub use crate::traits::{Item, NestedItem, Table};
    pub use aymond_derive::aymond;
}

pub mod condition;
pub mod error;
pub mod retry;
pub mod shim;
pub mod traits;
pub mod update;

pub struct Tx<'a> {
    client: &'a Aymond,
    transact_items: Vec<TransactWriteItem>,
}

pub struct Aymond {
    pub client: Arc<aws_sdk_dynamodb::Client>,
    pub retry_strategy: retry::RetryStrategy,
}

impl Clone for Aymond {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            retry_strategy: self.retry_strategy.clone(),
        }
    }
}

impl std::fmt::Debug for Aymond {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Aymond")
            .field("client", &self.client)
            .field("retry_strategy", &"<closure>")
            .finish()
    }
}

impl<'a> Aymond {
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
        Self {
            client,
            retry_strategy: retry::default_retry_strategy(),
        }
    }

    pub fn with_retry_strategy(mut self, s: retry::RetryStrategy) -> Self {
        self.retry_strategy = s;
        self
    }

    pub fn tx(&'a self) -> Tx<'a> {
        Tx {
            client: self,
            transact_items: vec![],
        }
    }
}

impl<'a> Tx<'a> {
    pub fn put(mut self, put: impl Into<Put>) -> Self {
        self.transact_items
            .push(TransactWriteItem::builder().put(put.into()).build());
        self
    }

    pub fn delete(mut self, delete: impl Into<Delete>) -> Self {
        self.transact_items
            .push(TransactWriteItem::builder().delete(delete.into()).build());
        self
    }

    pub fn update(mut self, update: impl Into<Update>) -> Self {
        self.transact_items
            .push(TransactWriteItem::builder().update(update.into()).build());
        self
    }

    pub fn condition_check(mut self, condition_check: impl Into<ConditionCheck>) -> Self {
        self.transact_items.push(
            TransactWriteItem::builder()
                .condition_check(condition_check.into())
                .build(),
        );
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

impl<'a> From<Tx<'a>> for Option<Vec<TransactWriteItem>> {
    fn from(val: Tx<'a>) -> Self {
        Some(val.transact_items)
    }
}
