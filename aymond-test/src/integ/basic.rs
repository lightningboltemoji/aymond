#[tokio::test]
async fn test() {
    use aymond::{Aymond, prelude::*, shim::futures::StreamExt};

    #[aymond(item, table)]
    struct Car {
        #[aymond(hash_key)]
        make: String,
        #[aymond(sort_key)]
        model: String,
        hp: i16,
        variants: Vec<String>,
        production: Production,
    }

    #[aymond(nested_item)]
    struct Production {
        began: i32,
        #[aymond(attribute(name = "units_produced"))]
        units: i64,
    }

    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = CarTable::new(&aymond, "basic");
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
    table.put().item(it).send().await.expect("Failed to write");

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

    let res = table.query().make("Porsche").model_gt("9").send().await;
    let query: Vec<Car> = res.map(|e| e.ok().unwrap()).collect().await;
    assert_eq!(query, vec![it_factory()]);

    let res = table
        .query()
        .make("Porsche")
        .model_gt("9")
        .scan_index_forward(false)
        .consistent_read(true)
        .limit(1)
        .send()
        .await;
    let query: Vec<Car> = res.map(|e| e.ok().unwrap()).collect().await;
    assert_eq!(query, vec![it_factory()]);

    // ── Condition expression tests ──

    // Nested path condition: production.began > 1960 (true, should succeed)
    table
        .put()
        .item(it_factory())
        .condition(|c| c.production().began().gt(1960))
        .send()
        .await
        .expect("Nested path condition should succeed");

    // Nested path condition: production.units_produced >= 500_000 (true)
    table
        .put()
        .item(it_factory())
        .condition(|c| c.production().units().ge(500_000i64))
        .send()
        .await
        .expect("Nested units condition should succeed");

    // AND composition: make = "Porsche" AND hp > 500 (both true)
    table
        .put()
        .item(it_factory())
        .condition(|c| c.make().eq("Porsche").and(c.hp().gt(500i16)))
        .send()
        .await
        .expect("AND condition should succeed");

    // AND with false right side should fail: make = "Porsche" AND hp > 9000
    let result = table
        .put()
        .item(it_factory())
        .condition(|c| c.make().eq("Porsche").and(c.hp().gt(9000i16)))
        .send()
        .await;
    assert!(result.is_err(), "AND with false RHS should fail");

    // OR composition: make = "Toyota" OR hp > 500 (second is true)
    table
        .put()
        .item(it_factory())
        .condition(|c| c.make().eq("Toyota").or(c.hp().gt(500i16)))
        .send()
        .await
        .expect("OR condition should succeed when one side is true");

    // NOT: NOT(make = "Ford") — true because make is "Porsche"
    table
        .put()
        .item(it_factory())
        .condition(|c| c.make().eq("Ford").not())
        .send()
        .await
        .expect("NOT condition should succeed");

    // NOT: NOT(make = "Porsche") — should fail
    let result = table
        .put()
        .item(it_factory())
        .condition(|c| c.make().eq("Porsche").not())
        .send()
        .await;
    assert!(result.is_err(), "NOT on true condition should fail");

    // Vec indexing: variants[0] = "Carrera" (true)
    table
        .put()
        .item(it_factory())
        .condition(|c| c.variants().index(0).eq("Carrera"))
        .send()
        .await
        .expect("Vec index condition should succeed");

    // Vec indexing: variants[0] = "wrong" (false, should fail)
    let result = table
        .put()
        .item(it_factory())
        .condition(|c| c.variants().index(0).eq("wrong"))
        .send()
        .await;
    assert!(result.is_err(), "Vec index with wrong value should fail");
}
