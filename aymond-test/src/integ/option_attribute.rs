#[tokio::test]
async fn test() {
    use aymond::{HighLevelClient, prelude::*};

    #[aymond(item, table)]
    struct Cell {
        #[hash_key]
        row: i32,
        #[sort_key]
        col: i32,
        label: Option<String>,
    }

    let client = HighLevelClient::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = CellTable::new(&client, "option_attribute");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    let it_factory = || Cell {
        row: 10,
        col: 14,
        label: None,
    };
    let it = it_factory();
    table.put().item(it).send().await.expect("Failed to write");

    let get = table.get().row(10).col(14).send().await.unwrap();
    assert_eq!(get.unwrap(), it_factory());

    let it_factory = || Cell {
        row: 10,
        col: 14,
        label: Some("Red".to_string()),
    };
    let it = it_factory();
    table.put().item(it).send().await.expect("Failed to write");

    let get = table.get().row(10).col(14).send().await.unwrap();
    assert_eq!(get.unwrap(), it_factory());
}
