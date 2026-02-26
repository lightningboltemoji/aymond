#[tokio::test]
async fn test_update_item_with_expression_and_condition() {
    use aymond::{Aymond, prelude::*};
    use std::collections::HashSet;

    #[aymond(item, table)]
    struct Car {
        #[aymond(hash_key)]
        make: String,
        #[aymond(sort_key)]
        model: String,
        count: i32,
        flag: Option<String>,
        labels: HashSet<String>,
    }

    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = CarTable::new(&aymond, "update_item_basic");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    table
        .put()
        .item(Car {
            make: "Porsche".to_string(),
            model: "911".to_string(),
            count: 5,
            flag: Some("yes".to_string()),
            labels: HashSet::from(["sport".to_string(), "legacy".to_string()]),
        })
        .send()
        .await
        .expect("Failed to write");

    table
        .update()
        .make("Porsche")
        .model("911")
        .expression(|e| {
            e.count()
                .add(10)
                .and(e.remove().flag())
                .and(e.labels().delete("legacy"))
        })
        .condition(|c| c.count().eq(5))
        .send()
        .await
        .expect("Update should succeed");

    let res = table
        .get()
        .make("Porsche")
        .model("911")
        .send()
        .await
        .expect("Get should succeed")
        .expect("Item should exist");
    assert_eq!(res.count, 15);
    assert_eq!(res.flag, None);
    assert_eq!(res.labels, HashSet::from(["sport".to_string()]));
}

#[tokio::test]
async fn test_update_item_hash_key_only_table() {
    use aymond::{Aymond, prelude::*};

    #[aymond(item, table)]
    struct Counter {
        #[aymond(hash_key)]
        id: String,
        value: i32,
    }

    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = CounterTable::new(&aymond, "update_item_hash_only");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    table
        .put()
        .item(Counter {
            id: "c1".to_string(),
            value: 1,
        })
        .send()
        .await
        .expect("Failed to write");

    table
        .update()
        .id("c1")
        .expression(|e| e.value().add(41))
        .send()
        .await
        .expect("Update should succeed");

    let res = table
        .get()
        .id("c1")
        .send()
        .await
        .expect("Get should succeed")
        .expect("Item should exist");
    assert_eq!(res.value, 42);
}
