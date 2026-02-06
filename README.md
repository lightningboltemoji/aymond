# aymond

A batteries-included client wrapper for DynamoDB

Builds upon the existing AWS SDK DynamoDB client, providing a high-level interface, somewhat akin to the [DynamoDB Enhanced Java Client](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/DynamoDBEnhanced.html)

## Usage

Item shapes and table schemas are described by structs. 
* The `item` and `nested_item` macros will generate serialization logic and helper functions. 
* The `table` macro will generate high-level client methods (e.g. get, put).

```rust
use aymond::prelude::*;

#[nested_item]
struct Production {
    began: i32,
    #[attribute(name = "units_produced")]
    units: i64,
}

#[item]
struct Car {
    #[hash_key]
    make: String,
    #[sort_key]
    model: String,
    hp: i16,
    production: Production,
    variants: Vec<String>,
}

#[table(Car)]
struct CarTable {}
```

Interacting with the table is done through a `Table` instance. The appropriate CreateTable request is automatically generated based on the item schema.

```rust
let table = CarTable::new_with_local_config("test", "http://localhost:8000", "us-west-2");
table.create(false).await.expect("Failed to create");
```

Writing an item with `put`

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

Reading an item with `get`. A key function is automatically created based on item schema.

```rust
let key = Car::key("Porsche", "911");
let _: Option<Car> = table.get(key.clone()).await.expect("Failed to read");
```

<details>
    <summary>Full example</summary>

```rust
use aymond::prelude::*;

#[nested_item]
struct Production {
    began: i32,
    #[attribute(name = "units_produced")]
    units: i64,
}

#[item]
struct Car {
    #[hash_key]
    make: String,
    #[sort_key]
    model: String,
    hp: i16,
    production: Production,
    variants: Vec<String>,
}

#[table(Car)]
struct CarTable {}

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

    // Read it back!
    let key = Car::key("Porsche", "911");
    let _: Option<Car> = table.get(key.clone()).await.expect("Failed to read");
}
```
</details>
