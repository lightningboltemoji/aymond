#[tokio::test]
async fn test() {
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

    let req = table.get().make("Porsche").model("911");
    let get = req.send().await.unwrap();
    assert_eq!(get.unwrap(), it_factory());

    let req = table.get().make("Porsche").model("911");
    let res = req.raw(|r| r.consistent_read(true)).await.ok().unwrap();
    assert_eq!(
        res.item().unwrap()["production"].as_m().unwrap()["units_produced"]
            .as_n()
            .unwrap(),
        "1100000"
    );
    let get: Option<Car> = res.item().map(|i| i.into());
    assert_eq!(get.unwrap(), it_factory());

    let res = table.query(|q| q.make("Porsche").model_gt("9"));
    let query: Vec<Car> = res.map(|e| e.ok().unwrap()).collect().await;
    assert_eq!(query, vec![it_factory()]);

    let res = table
        .query_ext(
            |q| q.make("Porsche").model_gt("9"),
            |r| r.scan_index_forward(false),
        )
        .await;
    let query: Vec<Car> = res.unwrap().items().iter().map(|e| e.into()).collect();
    assert_eq!(query, vec![it_factory()]);
}
