#[tokio::test]
async fn basic() {
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

    let table = CarTable::new_with_local_config("basic", "http://localhost:8000", "us-west-2");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    let it_factory = || Car {
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
    let it = it_factory();
    table.put(it).await.expect("Failed to write");

    let get = table.get(|k| k.make("Porsche").model("911")).await.unwrap();
    assert!(get.unwrap() == it_factory());

    let res = table
        .get_item(
            |k| k.make("Porsche").model("911"),
            |r| r.consistent_read(true),
        )
        .await;
    let get: Option<Car> = res.ok().and_then(|e| e.item().map(|i| i.into()));
    assert!(get.unwrap() == it_factory());

    let res = table.query(|q| q.make("Porsche").model_gt("9"));
    let query: Vec<Car> = res.map(|e| e.ok().unwrap()).collect().await;
    assert!(query == vec![it_factory()]);

    let res = table
        .query_ext(
            |q| q.make("Porsche").model_gt("9"),
            |r| r.scan_index_forward(false),
        )
        .await;
    let query: Vec<Car> = res.unwrap().items().iter().map(|e| e.into()).collect();
    assert!(query == vec![it_factory()]);
}

#[tokio::test]
async fn no_sort_key() {
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

    let get = table.get(|k| k.make("Porsche")).await.unwrap();
    assert!(get.unwrap() == it_factory());

    let res = table
        .get_item(|k| k.make("Porsche"), |r| r.consistent_read(true))
        .await;
    let get: Option<Car> = res.ok().and_then(|e| e.item().map(|i| i.into()));
    assert!(get.unwrap() == it_factory());

    let res = table.query(|q| q.make("Porsche"));
    let query: Vec<Car> = res.map(|e| e.ok().unwrap()).collect().await;
    assert!(query == vec![it_factory()]);

    let res = table.query_ext(|q| q.make("Porsche"), |r| r.limit(1)).await;
    let query: Vec<Car> = res.unwrap().items().iter().map(|e| e.into()).collect();
    assert!(query == vec![it_factory()]);
}

#[tokio::test]
async fn numeric_keys() {
    use aymond::{prelude::*, shim::futures::StreamExt};

    #[aymond(item, table)]
    struct Cell {
        #[hash_key]
        row: i32,
        #[sort_key]
        col: i32,
    }

    let table =
        CellTable::new_with_local_config("numeric_keys", "http://localhost:8000", "us-west-2");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    let it_factory = || Cell { row: 10, col: 14 };
    let it = it_factory();
    table.put(it).await.expect("Failed to write");

    let get = table.get(|k| k.row(10).col(14)).await.unwrap();
    assert!(get.unwrap() == it_factory());

    let res: Result<_, _> = table
        .get_item(|k| k.row(10).col(14), |r| r.consistent_read(true))
        .await;
    let get: Option<Cell> = res.ok().and_then(|e| e.item().map(|i| i.into()));
    assert!(get.unwrap() == it_factory());

    let res = table.query(|q| q.row(10).col_gt(10));
    let query: Vec<Cell> = res.map(|e| e.ok().unwrap()).collect().await;
    assert!(query == vec![it_factory()]);

    let res = table
        .query_ext(|q| q.row(10).col_gt(10), |r| r.scan_index_forward(false))
        .await;
    let query: Vec<Cell> = res.unwrap().items().iter().map(|e| e.into()).collect();
    assert!(query == vec![it_factory()]);
}

#[test]
fn compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("src/shouldnt_compile/*.rs");
}
