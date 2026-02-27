# aymond

_A batteries-included client wrapper for DynamoDB_

Builds upon the existing AWS SDK DynamoDB client, providing a high-level interface, somewhat akin to the [DynamoDB Enhanced Java Client](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/DynamoDBEnhanced.html). Utilizes code generation to create tailored interfaces for items, pushing as much validation as possible to compile time.

## Quickstart

Items are described by structs:

```rust
#[aymond(item, table)]
struct Car {
    #[aymond(hash_key)]
    make: String,
    #[aymond(sort_key)]
    model: String,
    hp: i16,
}
```

Table instances are used for interactions:

```rust
let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
let table = CarTable::new(&aymond, "my-table-name");

// Create a table in local DynamoDB, based on our item schema
table.create(false).await.expect("Failed to create");
```

Write items with `put`:

```rust
let it = Car {
    make: "Porsche".to_string(),
    model: "911".to_string(),
    hp: 518,
};
table.put().item(it).send().await.expect("Failed to write");
```

Read items with `get`, `query`, `scan` (and more!):

```rust
let req = table.get().make("Porsche").model("911");
let _: Option<Car> = req.send().await.expect("Failed to read");
```

## Usage

### Attribute types

aymond maps each attribute's Rust type to the corresponding DynamoDB type

#### Scalars

|Rust|DynamoDB|
|-|-|
|String|AttributeValue::S|
|i32|AttributeValue::N|
|Vec\<u8\>|AttributeValue::B|
|HashSet\<String\>|AttributeValue::Ss|
|HashSet\<Vec\<u8\>\>|AttributeValue::Bs|
|Vec\<String\>|AttributeValue::L|
|Nested items|AttributeValue::M|

#### Nested items

```rust
#[aymond(item, table)]
struct Student {
    #[aymond(hash_key)]
    name: String,
    grades: Grades,
}

#[aymond(nested_item)]
struct Grades {
    fall: i32,
    winter: i32,
    spring: i32,
}
```

#### Optional attributes

By default, attributes are treated as required. To make an attribute optional, use the `Option` type:

```
#[aymond(item, table)]
struct Student {
    #[aymond(hash_key)]
    name: String,
    grades: Option<Grades>,
}
```

### Operations

Most relevant [DynamoDB actions](https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_Operations.html) should be implemented. Below is based on the `Car` struct from quickstart. Since function names are code-generated from an item's attributes, examples can't be entirely generic.

#### Get

```rust
let req = table.get().make("Porsche").model("911");
let _: Option<Car> = req.send().await.unwrap();
```

#### Put

```rust
let it = Car {
    make: "Porsche".to_string(),
    model: "911".to_string(),
    hp: 518,
};
table.put().item(it).send().await.unwrap();
```

#### Query

```rust
let req = table.query().make("Porsche").model_begins_with("9");
let _: Vec<Car> = req.send().await.map(|e| e.ok().unwrap()).collect().await;
```

#### Scan

```rust
let req = table.scan();
let _: Vec<Car> = req.send().await.map(|e| e.ok().unwrap()).collect().await;
```

#### Update

```rust
let _: Result<(), _> = table
    .update()
    .make("Porsche")
    .model("911")
    .expression(|e| e.hp().set(541i16))
    .send()
    .await;
```

#### Batch get

```rust
let _: Vec<Car> = table
    .batch_get()
    .make_and_model("Porsche", "911")
    .make_and_model("Honda", "Civic")
    .send()
    .await
    .unwrap();
```

#### Batch write

```rust
let _: Result<(), _> = table
    .batch_write()
    .put(Car {
        make: "Honda".to_string(),
        model: "Civic".to_string(),
        hp: 150,
    })
    .delete()
    .make("Porsche")
    .model("911")
    .send()
    .await;
```

### Advanced features

#### Transactions

The `Aymond` instance can be used to build and send transactions using TransactWriteItems. These can span tables and use the same builders as individual requests:

```rust
let _: Result<(), _> = aymond
    .tx()
    .update(
        table
            .update()
            .make("Honda")
            .model("Civic")
            .expression(|e| e.hp().set(200i16)),
    )
    .put(table.put().item(Car {
        make: "Tesla".to_string(),
        model: "Model Y".to_string(),
        hp: 460,
    }))
    .delete(table.delete_item().make("Porsche").model("911"))
    .send()
    .await;
```

#### Optimistic locking

Items can define a `#[aymond(version)]` attribute:

```
#[aymond(item, table)]
struct Item {
    #[aymond(hash_key)]
    id: String,
    #[aymond(version)]
    ver: i3,
}
```

When set, operations like `table.delete().item(<>)` and `table.put().item(<>)` will enforce version checking. 

In the case of `put()`, the version number will be incremented during write -- for example, if the input to `put()` had `ver: 6`, we'd generate a condition expression that ensures DynamoDB _currently_ has 6 and overwrite it with 7. Version 0 is treated as a sentinel value that ensures object creation.

If you want to bypass versioning on a specific request, you can do that with a condition expression -- `table.put().item(<>).condition(|c| c.disable_versioning())`.

#### Condition/update expressions

Both types of expressions support:
- Deep nesting with list and map access
- Type awareness: string properties will have a `begins_with` method while numeric types wont

To illustrate, take for example this item:

```rust
#[aymond(item, table)]
struct Person {
    #[aymond(hash_key)]
    name: String,
    address: Address,
    ssn: Vec<i32>,
}

#[aymond(nested_item)]
struct Address {
    street: String,
    city: String,
    state: String,
}
```

Expressions like these could be used, seeking into both lists and nested items:

```rust
table
    .update()
    .name("John Doe")
    .expression(|e| {
        e.address().city().set("Seattle")
            .and(e.address().state().set("WA"))
            .and(e.phone().index(2).add(3))
    })
    .condition(|e| {
        e.address().street().begins_with("123")
            .and(e.phone().index(0).gt(100))
    })
```

## Development

The tests assume that DynamoDB local is available on port 8000 -- start it with any container runtime:

```bash
container run --name dynamodb-local -d -p 8000:8000 amazon/dynamodb-local
```

The integration tests can be ran with:

```bash
cargo test -p aymond-test
```
