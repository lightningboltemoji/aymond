#[tokio::test]
async fn test() {
    use aymond::{prelude::*, shim::futures::StreamExt};

    #[aymond(item, table)]
    struct Car {
        #[hash_key]
        make: String,
        hp: i16,
    }

    let table =
        CarTable::new_with_local_config("no_sort_key", "http://localhost:8000", "us-west-2");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    let it_factory = || Car {
        make: "Porsche".to_string(),
        hp: 518,
    };
    let it = it_factory();
    table.put(it).await.expect("Failed to write");

    let get = table.get().make("Porsche").send().await.unwrap();
    assert_eq!(get.unwrap(), it_factory());

    let req = table.get().make("Porsche");
    let res = req.raw(|r| r.consistent_read(true)).await;
    let get: Option<Car> = res.ok().and_then(|e| e.item().map(|i| i.into()));
    assert_eq!(get.unwrap(), it_factory());

    let res = table.query().make("Porsche").send().await;
    let query: Vec<Car> = res.map(|e| e.ok().unwrap()).collect().await;
    assert_eq!(query, vec![it_factory()]);

    let res = table.query().make("Porsche").raw(|r| r.limit(1)).await;
    let query: Vec<Car> = res.unwrap().items().iter().map(|e| e.into()).collect();
    assert_eq!(query, vec![it_factory()]);
}
