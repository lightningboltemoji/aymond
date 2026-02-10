use aymond::{prelude::*, shim::futures::StreamExt};

#[aymond(item, table)]
struct Car {
    #[hash_key]
    make: String,
    #[sort_key]
    model: String,
    hp: i16,
    variants: Vec<String>,
    production: Production,
}

#[aymond(nested_item)]
struct Production {
    began: i32,
    #[attribute(name = "units_produced")]
    units: i64,
}

#[tokio::main]
async fn main() {
    // Create a table in local DynamoDB, based on our item schema
    let table = CarTable::new_with_local_config("test", "http://localhost:8000", "us-west-2");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

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
    table.put(it).await.expect("Failed to write");

    // Read it back!
    let key = Car::key("Porsche", "911");
    let _: Option<Car> = table.get(key.clone()).await.expect("Failed to read");

    // Read it back, with additional options
    let res: Result<_, _> = table.get_item(key, |r| r.consistent_read(true)).await;
    let _: Option<Car> = res.ok().and_then(|e| e.item().map(|i| i.into()));

    // Query
    let res = table.query(|q| q.make("Porsche").model_gt("9"));
    let _: Vec<Car> = res.map(|e| e.ok().unwrap()).collect().await;

    // Query, with additional options
    let _: Result<_, _> = table
        .query_ext(|q| q.make("Porsche").model_gt("8"), |r| r)
        .await;
}

#[test]
fn compile() {
    let t = trybuild::TestCases::new();
    t.pass("src/should_compile/*.rs");
    t.compile_fail("src/shouldnt_compile/*.rs");
}
