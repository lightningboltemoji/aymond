#[tokio::test]
async fn test_batch_write_puts_with_sort_key() {
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
    let table = CarTable::new(&aymond, "batch_write_puts_sort");
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

    table
        .batch_write()
        .put(porsche())
        .put(bmw())
        .put(ferrari())
        .send()
        .await
        .unwrap();

    let p = table
        .get()
        .make("Porsche")
        .model("911")
        .send()
        .await
        .unwrap();
    assert_eq!(p, Some(porsche()));

    let b = table.get().make("BMW").model("M3").send().await.unwrap();
    assert_eq!(b, Some(bmw()));

    let f = table
        .get()
        .make("Ferrari")
        .model("F40")
        .send()
        .await
        .unwrap();
    assert_eq!(f, Some(ferrari()));
}

#[tokio::test]
async fn test_batch_write_deletes_with_sort_key() {
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
    let table = CarTable::new(&aymond, "batch_write_deletes_sort");
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

    table.put().item(porsche()).send().await.unwrap();
    table.put().item(bmw()).send().await.unwrap();

    table
        .batch_write()
        .delete()
        .make("Porsche")
        .model("911")
        .delete()
        .make("BMW")
        .model("M3")
        .send()
        .await
        .unwrap();

    let p = table
        .get()
        .make("Porsche")
        .model("911")
        .send()
        .await
        .unwrap();
    assert_eq!(p, None);

    let b = table.get().make("BMW").model("M3").send().await.unwrap();
    assert_eq!(b, None);
}

#[tokio::test]
async fn test_batch_write_mixed_put_and_delete() {
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
    let table = CarTable::new(&aymond, "batch_write_mixed_sort");
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

    // Pre-insert porsche so we can delete it in the batch
    table.put().item(porsche()).send().await.unwrap();

    table
        .batch_write()
        .put(bmw())
        .put(ferrari())
        .delete()
        .make("Porsche")
        .model("911")
        .send()
        .await
        .unwrap();

    let p = table
        .get()
        .make("Porsche")
        .model("911")
        .send()
        .await
        .unwrap();
    assert_eq!(p, None);

    let b = table.get().make("BMW").model("M3").send().await.unwrap();
    assert_eq!(b, Some(bmw()));

    let f = table
        .get()
        .make("Ferrari")
        .model("F40")
        .send()
        .await
        .unwrap();
    assert_eq!(f, Some(ferrari()));
}

#[tokio::test]
async fn test_batch_write_no_sort_key() {
    use aymond::{Aymond, prelude::*};

    #[aymond(item, table)]
    struct Maker {
        #[aymond(hash_key)]
        name: String,
        country: String,
    }

    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = MakerTable::new(&aymond, "batch_write_no_sort");
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
    let toyota = || Maker {
        name: "Toyota".into(),
        country: "Japan".into(),
    };

    // Batch put
    table
        .batch_write()
        .put(porsche())
        .put(ferrari())
        .put(toyota())
        .send()
        .await
        .unwrap();

    let p = table.get().name("Porsche").send().await.unwrap();
    assert_eq!(p, Some(porsche()));

    let f = table.get().name("Ferrari").send().await.unwrap();
    assert_eq!(f, Some(ferrari()));

    // Batch delete
    table
        .batch_write()
        .delete()
        .name("Porsche")
        .delete()
        .name("Ferrari")
        .send()
        .await
        .unwrap();

    let p = table.get().name("Porsche").send().await.unwrap();
    assert_eq!(p, None);

    let f = table.get().name("Ferrari").send().await.unwrap();
    assert_eq!(f, None);

    // Toyota should still exist
    let t = table.get().name("Toyota").send().await.unwrap();
    assert_eq!(t, Some(toyota()));
}
