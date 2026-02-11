# aymond

A batteries-included client wrapper for DynamoDB

Builds upon the existing AWS SDK DynamoDB client, providing a high-level interface, somewhat akin to the [DynamoDB Enhanced Java Client](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/DynamoDBEnhanced.html)

## Usage

Item shapes are described by structs:

```rust
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
```

Interacting with the table is done through a `Table` instance:

```rust
let table = CarTable::new_with_local_config("test", "http://localhost:8000", "us-west-2");
table.create(false).await.expect("Failed to create");
```

Writing an item with `put`:

```rust
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
```

Reading an item with `get`:

```rust
let _: Option<Car> = table
    .get(|k| k.make("Porsche").model("911"))
    .await
    .expect("Failed to read");
```

<details>
    <summary>Full example</summary>

```rust
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

    // Read it back
    let _: Option<Car> = table
        .get(|k| k.make("Porsche").model("911"))
        .await
        .expect("Failed to read");

    // Read it back, with additional options
    let res: Result<_, _> = table
        .get_item(
            |k| k.make("Porsche").model("911"),
            |r| r.consistent_read(true),
        )
        .await;
    let _: Option<Car> = res.ok().and_then(|e| e.item().map(|i| i.into()));
}
```
</details>

## Not (yet) implemented

- Query operation with fluent builder pattern
- Update operation with fluent builder pattern
- Global/local indices
- Optimistic locking with version attribute
- Transaction system, including cross-table
- Projections

## Development

The tests assume that DynamoDB local is available on port 8000 -- start it with any container runtime:

```bash
container run --name dynamodb-local -d -p 8000:8000 amazon/dynamodb-local
```

The integration tests can be ran with:

```bash
cargo run -p aymond-test
```
