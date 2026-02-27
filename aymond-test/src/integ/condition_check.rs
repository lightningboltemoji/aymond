#[tokio::test]
async fn test_condition_check_in_transaction() {
    use aymond::{Aymond, prelude::*};

    #[aymond(item, table)]
    struct Car {
        #[aymond(hash_key)]
        make: String,
        #[aymond(sort_key)]
        model: String,
        hp: i16,
    }

    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = CarTable::new(&aymond, "condition_check_tx");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    table
        .put()
        .item(Car {
            make: "Porsche".to_string(),
            model: "911".to_string(),
            hp: 518,
        })
        .send()
        .await
        .expect("Failed to seed");

    aymond
        .tx()
        .condition_check(
            table
                .condition_check()
                .make("Porsche")
                .model("911")
                .condition(|c| c.hp().eq(518i16)),
        )
        .put(table.put().item(Car {
            make: "BMW".to_string(),
            model: "M3".to_string(),
            hp: 473,
        }))
        .send()
        .await
        .expect("Condition check should pass");

    let bmw = table.get().make("BMW").model("M3").send().await.unwrap();
    assert!(bmw.is_some());

    let failed = aymond
        .tx()
        .condition_check(
            table
                .condition_check()
                .make("Porsche")
                .model("911")
                .condition(|c| c.hp().eq(999i16)),
        )
        .put(table.put().item(Car {
            make: "Audi".to_string(),
            model: "R8".to_string(),
            hp: 602,
        }))
        .send()
        .await;
    assert!(failed.is_err(), "Condition check should fail");

    let audi = table.get().make("Audi").model("R8").send().await.unwrap();
    assert!(audi.is_none(), "Failed transaction should not write item");
}
