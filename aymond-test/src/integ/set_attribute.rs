#[tokio::test]
async fn test() {
    use aymond::{HighLevelClient, prelude::*};
    use std::collections::HashSet;

    #[aymond(item, table)]
    struct Tag {
        #[aymond(hash_key)]
        pk: String,
        labels: HashSet<String>,
        blobs: HashSet<Vec<u8>>,
        extra: Option<HashSet<String>>,
    }

    let client = HighLevelClient::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = TagTable::new(&client, "set_attribute");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    let it_factory = || Tag {
        pk: "row1".to_string(),
        labels: HashSet::from(["rust".to_string(), "backend".to_string()]),
        blobs: HashSet::from([vec![1u8, 2, 3]]),
        extra: None,
    };
    table.put().item(it_factory()).send().await.expect("Failed to write");

    // Round-trip get (None extra)
    let get = table.get().pk("row1").send().await.unwrap();
    assert_eq!(get.unwrap(), it_factory());

    // Round-trip with Some(extra)
    let it2_factory = || Tag {
        pk: "row2".to_string(),
        labels: HashSet::from(["java".to_string()]),
        blobs: HashSet::from([vec![9u8]]),
        extra: Some(HashSet::from(["opt".to_string()])),
    };
    table.put().item(it2_factory()).send().await.expect("Failed to write");
    let get = table.get().pk("row2").send().await.unwrap();
    assert_eq!(get.unwrap(), it2_factory());

    // Condition expression: put only if labels contains "rust"
    let it3 = Tag {
        pk: "row3".to_string(),
        labels: HashSet::from(["rust".to_string()]),
        blobs: HashSet::from([vec![0u8]]),
        extra: None,
    };
    table.put().item(it3).send().await.expect("Failed to write");

    // This should fail: condition "labels contains java" is false
    let result = table
        .put()
        .item(Tag {
            pk: "row3".to_string(),
            labels: HashSet::from(["other".to_string()]),
            blobs: HashSet::from([vec![0u8]]),
            extra: None,
        })
        .condition(|c| c.labels().contains("java"))
        .send()
        .await;
    assert!(result.is_err());
}
