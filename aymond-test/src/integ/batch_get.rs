#[tokio::test]
async fn test_batch_get_with_sort_key() {
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
    let table = CarTable::new(&aymond, "batch_get_sort");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    let porsche = || Car {
        make: "Porsche".into(),
        model: "911".into(),
        hp: 518,
    };
    let bmw = || Car {
        make: "BMW".into(),
        model: "M3".into(),
        hp: 473,
    };
    let ferrari = || Car {
        make: "Ferrari".into(),
        model: "F40".into(),
        hp: 471,
    };

    table.put().item(porsche()).send().await.unwrap();
    table.put().item(bmw()).send().await.unwrap();
    table.put().item(ferrari()).send().await.unwrap();

    // Batch get 2 of 3 items
    let mut results = table
        .batch_get()
        .make_and_model("Porsche", "911")
        .make_and_model("BMW", "M3")
        .send()
        .await
        .unwrap();
    results.sort_by(|a, b| a.make.cmp(&b.make));

    assert_eq!(results.len(), 2);
    assert_eq!(results[0], bmw());
    assert_eq!(results[1], porsche());

    // Batch get with non-existent key
    let results = table
        .batch_get()
        .make_and_model("Toyota", "Supra")
        .send()
        .await
        .unwrap();
    assert!(results.is_empty());

    // Batch get mixing existent and non-existent
    let results = table
        .batch_get()
        .make_and_model("Ferrari", "F40")
        .make_and_model("Toyota", "Supra")
        .send()
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], ferrari());
}

#[tokio::test]
async fn test_batch_get_no_sort_key() {
    use aymond::{Aymond, prelude::*};

    #[aymond(item, table)]
    struct Maker {
        #[aymond(hash_key)]
        name: String,
        country: String,
    }

    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = MakerTable::new(&aymond, "batch_get_no_sort");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    let porsche = || Maker {
        name: "Porsche".into(),
        country: "Germany".into(),
    };
    let ferrari = || Maker {
        name: "Ferrari".into(),
        country: "Italy".into(),
    };

    table.put().item(porsche()).send().await.unwrap();
    table.put().item(ferrari()).send().await.unwrap();

    let mut results = table
        .batch_get()
        .name("Porsche")
        .name("Ferrari")
        .send()
        .await
        .unwrap();
    results.sort_by(|a, b| a.name.cmp(&b.name));

    assert_eq!(results.len(), 2);
    assert_eq!(results[0], ferrari());
    assert_eq!(results[1], porsche());
}
