#[tokio::test]
async fn test() {
    use aymond::{HighLevelClient, prelude::*, shim::futures::StreamExt};

    #[aymond(item, table)]
    #[derive(Clone)]
    struct Widget {
        #[aymond(hash_key)]
        id: String,
        value: i32,
    }

    let client = HighLevelClient::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = WidgetTable::new(&client, "scan");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    let items = vec![
        Widget {
            id: "a".to_string(),
            value: 1,
        },
        Widget {
            id: "b".to_string(),
            value: 2,
        },
        Widget {
            id: "c".to_string(),
            value: 3,
        },
    ];
    for item in items.clone() {
        table
            .put()
            .item(item)
            .send()
            .await
            .expect("Failed to write");
    }

    let stream = table.scan().send().await;
    let mut results: Vec<Widget> = stream.map(|e| e.ok().unwrap()).collect().await;
    results.sort_by(|a, b| a.id.cmp(&b.id));

    assert_eq!(results, items);
}
