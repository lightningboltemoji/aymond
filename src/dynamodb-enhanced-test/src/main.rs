use dynamodb_enhanced::prelude::*;

#[nested_item]
struct Production {
    #[attribute(name = "Began")]
    began: i32,
    #[attribute(name = "Units")]
    units: i64,
}

#[item]
struct Car {
    #[hash_key(name = "Make")]
    make: String,
    #[sort_key(name = "Model")]
    model: String,
    #[attribute(name = "Horsepower")]
    hp: i16,
    #[attribute(name = "Production")]
    production: Production,
    #[attribute(name = "Variants")]
    variants: Vec<String>,
}

#[table(Car)]
struct CarTable {}

#[tokio::main]
async fn main() {
    // Create a table in local DynamoDB, based on our item schema
    let table = CarTable::new_with_local_config("test", "http://localhost:8000", "us-west-2");
    table.create_table(false).await.expect("Failed to create");

    // Write
    let it = Car {
        make: "Porsche".to_string(),
        model: "911".to_string(),
        hp: 518,
        variants: vec![
            "Carrera".into(),
            "Carrera S".into(),
            "Carrera 4S".into(),
            "GT3 RS".into(),
        ],
        production: Production {
            began: 1964,
            units: 1_100_000,
        },
    };
    table.put_item(it).await.expect("Failed to write");

    // Read it back!
    let key = Car::key("Porsche".into(), "911".into());
    let res = table.get_item(key).await.expect("Failed to read");
    let _: Car = res.item().unwrap().into();
}
