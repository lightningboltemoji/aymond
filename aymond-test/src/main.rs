use aymond::{Aymond, prelude::*, shim::futures::StreamExt};

mod integ;

#[aymond(item, table)]
struct Car {
    #[aymond(hash_key)]
    make: String,
    #[aymond(sort_key)]
    model: String,
    hp: i16,
}

#[aymond(item, table)]
struct Person {
    #[aymond(hash_key)]
    name: String,
    address: Address,
    phone: Vec<i32>,
}

#[aymond(nested_item)]
struct Address {
    street: String,
    city: String,
    state: String,
}

#[tokio::main]
async fn main() {
    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = CarTable::new(&aymond, "my-table-name");

    // Create a table in local DynamoDB, based on our item schema
    table.create(false).await.expect("Failed to create");

    let it = Car {
        make: "Porsche".to_string(),
        model: "911".to_string(),
        hp: 518,
    };
    table.put().item(it).send().await.expect("Failed to write");

    let req = table.get().make("Porsche").model("911");
    let _: Option<Car> = req.send().await.unwrap();

    let req = table.query().make("Porsche").model_begins_with("9");
    let _: Vec<Car> = req.send().await.map(|e| e.ok().unwrap()).collect().await;

    let req = table.scan();
    let _: Vec<Car> = req.send().await.map(|e| e.ok().unwrap()).collect().await;

    let _: Result<(), _> = table
        .update()
        .make("Porsche")
        .model("911")
        .expression(|e| e.hp().set(541i16))
        .send()
        .await;

    let _: Vec<Car> = table
        .batch_get()
        .make_and_model("Porsche", "911")
        .make_and_model("Honda", "Civic")
        .send()
        .await
        .unwrap();

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

    let table = PersonTable::new(&aymond, "my-table-name2");
    table.create(false).await.expect("Failed to create");

    table
        .put()
        .item(Person {
            name: "John Doe".to_string(),
            address: Address {
                street: "123 Easy Street".to_string(),
                city: "Palm Beach".to_string(),
                state: "FL".to_string(),
            },
            phone: vec![111, 22, 3333],
        })
        .send()
        .await
        .unwrap();

    table
        .update()
        .name("John Doe")
        .expression(|e| {
            e.address()
                .city()
                .set("Seattle")
                .and(e.address().state().set("WA"))
                .and(e.phone().index(2).add(3))
        })
        .condition(|e| {
            e.address()
                .street()
                .begins_with("123")
                .and(e.phone().index(0).gt(100))
        })
        .send()
        .await
        .unwrap();
}
