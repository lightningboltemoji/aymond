use aymond::{HighLevelClient, prelude::*};

mod integ;

#[aymond(item, table)]
struct Cell {
    #[hash_key]
    row: i32,
    #[sort_key]
    col: i32,
    label: Option<String>,
}

#[tokio::main]
async fn main() {
    let client = HighLevelClient::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = CellTable::new(&client, "numeric_keys");
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

    let it_factory = |col: i32| Cell {
        row: 10,
        col,
        label: Some("Red".to_string()),
    };
    let put1 = table.put().item(it_factory(15));
    let put2 = table.put().item(it_factory(16));
    client.tx().put(put1).put(put2).send().await.unwrap();
}
