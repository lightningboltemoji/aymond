use dynamodb_enhanced::prelude::*;

#[item]
struct Car {
    #[hash_key(name = "Make")]
    make: String,
    #[sort_key(name = "Model")]
    model: String,
    #[attribute(name = "Horsepower")]
    hp: i128,
}

#[table(Car)]
struct CarTable {}

#[tokio::main]
async fn main() {
    let table = CarTable::new_with_local_config("test", "http://localhost:8000", "us-west-2");
    table.create_table(false).await.expect("Failed to create");

    let it = Car {
        make: "Porsche".to_string(),
        model: "911".to_string(),
        hp: 518,
    };
    table.put_item(it).await.unwrap();
}
