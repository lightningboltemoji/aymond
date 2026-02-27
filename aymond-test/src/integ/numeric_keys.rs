#[tokio::test]
async fn test() {
    use aymond::{Aymond, prelude::*, shim::futures::StreamExt};

    #[aymond(item, table)]
    struct Cell {
        #[aymond(hash_key)]
        row: i32,
        #[aymond(sort_key)]
        col: i32,
    }

    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = CellTable::new(&aymond, "numeric_keys");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    let it_factory = || Cell { row: 10, col: 14 };
    let it = it_factory();
    table.put().item(it).send().await.expect("Failed to write");

    let get = table.get().row(10).col(14).send().await.unwrap();
    assert_eq!(get.unwrap(), it_factory());

    let get = table
        .get()
        .row(10)
        .col(14)
        .consistent_read(true)
        .send()
        .await
        .unwrap();
    assert_eq!(get.unwrap(), it_factory());

    let res = table.query().row(10).col_gt(10).send().await;
    let query: Vec<Cell> = res.map(|e| e.ok().unwrap()).collect().await;
    assert_eq!(query, vec![it_factory()]);

    let res = table
        .query()
        .row(10)
        .col_gt(10)
        .scan_index_forward(false)
        .consistent_read(true)
        .limit(1)
        .send()
        .await;
    let query: Vec<Cell> = res.map(|e| e.ok().unwrap()).collect().await;
    assert_eq!(query, vec![it_factory()]);
}
