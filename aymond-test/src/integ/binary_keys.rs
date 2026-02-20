#[tokio::test]
async fn test() {
    use aymond::{HighLevelClient, prelude::*, shim::futures::StreamExt};

    #[aymond(item, table)]
    struct Chunk {
        #[hash_key]
        key: Vec<u8>,
        #[sort_key]
        range: Vec<u8>,
    }

    let client = HighLevelClient::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = ChunkTable::new(&client, "binary_keys");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    let it_factory = || Chunk { key: vec![1, 2, 3], range: vec![4, 5, 6] };
    let it = it_factory();
    table.put().item(it).send().await.expect("Failed to write");

    let get = table.get().key(vec![1, 2, 3]).range(vec![4, 5, 6]).send().await.unwrap();
    assert_eq!(get.unwrap(), it_factory());

    let res = table
        .query()
        .key(vec![1, 2, 3])
        .range_begins_with(vec![4, 5])
        .send()
        .await;
    let query: Vec<Chunk> = res.map(|e| e.ok().unwrap()).collect().await;
    assert_eq!(query, vec![it_factory()]);
}
