#[tokio::test]
async fn test() {
    use aymond::{HighLevelClient, prelude::*, shim::futures::StreamExt};

    #[aymond(item, table)]
    struct Cell {
        #[hash_key]
        row: i32,
        #[sort_key]
        col: i32,
    }

    let client = HighLevelClient::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = CellTable::new(&client, "numeric_keys");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    let it_factory = || Cell { row: 10, col: 14 };
    let it = it_factory();
    table.put().item(it).send().await.expect("Failed to write");

    let get = table.get().row(10).col(14).send().await.unwrap();
    assert_eq!(get.unwrap(), it_factory());

    let req = table.get().row(10).col(14);
    let res = req.raw(|r| r.consistent_read(true)).await;
    let get: Option<Cell> = res.ok().and_then(|e| e.item().map(|i| i.into()));
    assert_eq!(get.unwrap(), it_factory());

    let res = table.query().row(10).col_gt(10).send().await;
    let query: Vec<Cell> = res.map(|e| e.ok().unwrap()).collect().await;
    assert_eq!(query, vec![it_factory()]);

    let res = table
        .query()
        .row(10)
        .col_gt(10)
        .raw(|r| r.scan_index_forward(false))
        .await;
    let query: Vec<Cell> = res.unwrap().items().iter().map(|e| e.into()).collect();
    assert_eq!(query, vec![it_factory()]);
}
