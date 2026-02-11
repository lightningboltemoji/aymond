#[tokio::test]
async fn test() {
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
    assert_eq!(get.unwrap(), it_factory());

    let res: Result<_, _> = table
        .get_item(|k| k.row(10).col(14), |r| r.consistent_read(true))
        .await;
    let get: Option<Cell> = res.ok().and_then(|e| e.item().map(|i| i.into()));
    assert_eq!(get.unwrap(), it_factory());

    let res = table.query(|q| q.row(10).col_gt(10));
    let query: Vec<Cell> = res.map(|e| e.ok().unwrap()).collect().await;
    assert_eq!(query, vec![it_factory()]);

    let res = table
        .query_ext(|q| q.row(10).col_gt(10), |r| r.scan_index_forward(false))
        .await;
    let query: Vec<Cell> = res.unwrap().items().iter().map(|e| e.into()).collect();
    assert_eq!(query, vec![it_factory()]);
}
