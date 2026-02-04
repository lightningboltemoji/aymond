use std::sync::Arc;

use aws_credential_types::Credentials;
use aws_sdk_dynamodb::{config::BehaviorVersion, error::SdkError};
use aws_types::{SdkConfig, region::Region, sdk_config::SharedCredentialsProvider};
use dynamodb_enhanced::{Item, Table};
use dynamodb_enhanced_derive::{item, table};

#[item]
struct Car {
    #[hash_key(name = "Make")]
    make: String,
    #[sort_key(name = "Model")]
    model: String,
    #[attribute(name = "Count")]
    count: i128,
}

#[table(Car)]
struct CarTable {}

#[tokio::main]
async fn main() {
    let credentials = Credentials::from_keys("x", "x", None);
    let config = SdkConfig::builder()
        .credentials_provider(SharedCredentialsProvider::new(credentials))
        .region(Region::new("us-west-2"))
        .endpoint_url("http://localhost:8000")
        .behavior_version(BehaviorVersion::latest())
        .build();
    let ddb = Arc::new(aws_sdk_dynamodb::Client::new(&config));

    let table = CarTable::new(ddb.clone(), "test");
    match table.create_table().await {
        Ok(_) => println!("Table created"),
        Err(SdkError::ServiceError(context)) => {
            if context.err().is_resource_in_use_exception() {
                println!("Table already exists");
            } else {
                panic!("Unhandled error: {}", context.err());
            }
        }
        Err(e) => panic!("Unhandled error: {}", e),
    }

    let r = Car {
        make: "Porsche".to_string(),
        model: "911".to_string(),
        count: 100,
    };

    ddb.put_item()
        .table_name("test")
        .set_item(Some(r.into()))
        .send()
        .await
        .unwrap();
    println!("Wrote item");
}
