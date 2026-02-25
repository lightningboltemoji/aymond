#[tokio::test]
async fn test_delete_with_sort_key() {
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
    let table = CarTable::new(&aymond, "delete_item_sort_key");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    let it = Car {
        make: "Porsche".to_string(),
        model: "911".to_string(),
        hp: 518,
    };
    table.put().item(it).send().await.expect("Failed to write");

    // Verify it exists
    let get = table
        .get()
        .make("Porsche")
        .model("911")
        .send()
        .await
        .unwrap();
    assert!(get.is_some());

    // Delete it
    table
        .delete_item()
        .make("Porsche")
        .model("911")
        .send()
        .await
        .expect("Failed to delete item");

    // Verify it's gone
    let get = table
        .get()
        .make("Porsche")
        .model("911")
        .send()
        .await
        .unwrap();
    assert!(get.is_none());
}

#[tokio::test]
async fn test_delete_no_sort_key() {
    use aymond::{Aymond, prelude::*};

    #[aymond(item, table)]
    struct Widget {
        #[aymond(hash_key)]
        id: String,
        name: String,
    }

    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = WidgetTable::new(&aymond, "delete_item_no_sort_key");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    let it = Widget {
        id: "w1".to_string(),
        name: "Gear".to_string(),
    };
    table.put().item(it).send().await.expect("Failed to write");

    // Verify it exists
    let get = table.get().id("w1").send().await.unwrap();
    assert!(get.is_some());

    // Delete it
    table
        .delete_item()
        .id("w1")
        .send()
        .await
        .expect("Failed to delete item");

    // Verify it's gone
    let get = table.get().id("w1").send().await.unwrap();
    assert!(get.is_none());
}

#[tokio::test]
async fn test_delete_with_condition() {
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
    let table = CarTable::new(&aymond, "delete_item_condition");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    let it = Car {
        make: "Porsche".to_string(),
        model: "911".to_string(),
        hp: 518,
    };
    table.put().item(it).send().await.expect("Failed to write");

    // Delete with a condition that fails (hp > 9000)
    let result = table
        .delete_item()
        .make("Porsche")
        .model("911")
        .condition(|c| c.hp().gt(9000i16))
        .send()
        .await;
    assert!(
        result.is_err(),
        "Condition should fail when hp is not > 9000"
    );

    // Verify it still exists
    let get = table
        .get()
        .make("Porsche")
        .model("911")
        .send()
        .await
        .unwrap();
    assert!(get.is_some());

    // Delete with a condition that succeeds (hp > 500)
    table
        .delete_item()
        .make("Porsche")
        .model("911")
        .condition(|c| c.hp().gt(500i16))
        .send()
        .await
        .expect("Condition should succeed when hp > 500");

    // Verify it's gone
    let get = table
        .get()
        .make("Porsche")
        .model("911")
        .send()
        .await
        .unwrap();
    assert!(get.is_none());
}

#[tokio::test]
async fn test_delete_nonexistent_item() {
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
    let table = CarTable::new(&aymond, "delete_item_nonexistent");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    // Deleting a nonexistent item should succeed (DynamoDB behavior)
    table
        .delete_item()
        .make("DoesNotExist")
        .model("Nope")
        .send()
        .await
        .expect("Deleting nonexistent item should succeed");
}

#[tokio::test]
async fn test_delete_in_transaction() {
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
    let table = CarTable::new(&aymond, "delete_item_tx");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    // Put two items
    let car1 = Car {
        make: "Porsche".to_string(),
        model: "911".to_string(),
        hp: 518,
    };
    let car2 = Car {
        make: "Porsche".to_string(),
        model: "Cayenne".to_string(),
        hp: 340,
    };
    let put1 = table.put().item(car1);
    let put2 = table.put().item(car2);
    aymond.tx().put(put1).put(put2).send().await.unwrap();

    // Verify both exist
    let get1 = table
        .get()
        .make("Porsche")
        .model("911")
        .send()
        .await
        .unwrap();
    assert!(get1.is_some());
    let get2 = table
        .get()
        .make("Porsche")
        .model("Cayenne")
        .send()
        .await
        .unwrap();
    assert!(get2.is_some());

    // Delete both in a transaction
    let del1 = table.delete_item().make("Porsche").model("911");
    let del2 = table.delete_item().make("Porsche").model("Cayenne");
    aymond.tx().delete(del1).delete(del2).send().await.unwrap();

    // Verify both are gone
    let get1 = table
        .get()
        .make("Porsche")
        .model("911")
        .send()
        .await
        .unwrap();
    assert!(get1.is_none());
    let get2 = table
        .get()
        .make("Porsche")
        .model("Cayenne")
        .send()
        .await
        .unwrap();
    assert!(get2.is_none());
}

#[tokio::test]
async fn test_mixed_put_delete_transaction() {
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
    let table = CarTable::new(&aymond, "delete_item_mixed_tx");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    // Put an initial item
    let car = Car {
        make: "Porsche".to_string(),
        model: "911".to_string(),
        hp: 518,
    };
    table.put().item(car).send().await.expect("Failed to write");

    // In one transaction: delete the old item and put a new one
    let del = table.delete_item().make("Porsche").model("911");
    let new_car = Car {
        make: "BMW".to_string(),
        model: "M3".to_string(),
        hp: 473,
    };
    let put = table.put().item(new_car);
    aymond.tx().delete(del).put(put).send().await.unwrap();

    // Old item gone
    let get = table
        .get()
        .make("Porsche")
        .model("911")
        .send()
        .await
        .unwrap();
    assert!(get.is_none());

    // New item exists
    let get = table.get().make("BMW").model("M3").send().await.unwrap();
    assert!(get.is_some());
    assert_eq!(get.unwrap().hp, 473);
}
