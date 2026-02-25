#[tokio::test]
async fn test_create_with_secondary_indexes() {
    use aymond::{Aymond, prelude::*};

    #[aymond(item, table)]
    struct Order {
        #[aymond(hash_key)]
        customer_id: String,

        #[aymond(sort_key)]
        order_id: String,

        #[aymond(gsi("by-status", hash_key))]
        status: String,

        #[aymond(gsi("by-status", sort_key))]
        #[aymond(lsi("by-amount"))]
        amount: i64,
    }

    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = OrderTable::new(&aymond, "secondary-indexes-test");
    table.delete(false).await.expect("Failed to delete");
    table
        .create(false)
        .await
        .expect("Failed to create table with GSI and LSI");
}

#[tokio::test]
async fn test_query_gsi() {
    use aymond::{Aymond, prelude::*, shim::futures::StreamExt};

    #[aymond(item, table)]
    struct Order {
        #[aymond(hash_key)]
        customer_id: String,

        #[aymond(sort_key)]
        order_id: String,

        #[aymond(gsi("by-status", hash_key))]
        status: String,

        #[aymond(gsi("by-status", sort_key))]
        #[aymond(lsi("by-amount"))]
        amount: i64,
    }

    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = OrderTable::new(&aymond, "gsi-query-test");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    // Insert test data
    table
        .put()
        .item(Order {
            customer_id: "c1".into(),
            order_id: "o1".into(),
            status: "shipped".into(),
            amount: 100,
        })
        .send()
        .await
        .expect("Failed to put o1");

    table
        .put()
        .item(Order {
            customer_id: "c1".into(),
            order_id: "o2".into(),
            status: "pending".into(),
            amount: 50,
        })
        .send()
        .await
        .expect("Failed to put o2");

    table
        .put()
        .item(Order {
            customer_id: "c2".into(),
            order_id: "o3".into(),
            status: "shipped".into(),
            amount: 200,
        })
        .send()
        .await
        .expect("Failed to put o3");

    // Query GSI by-status: all "shipped" orders with amount > 50
    let results: Vec<Order> = table
        .query_by_status()
        .status("shipped")
        .amount_gt(50)
        .send()
        .await
        .map(|e| e.ok().unwrap())
        .collect()
        .await;

    assert_eq!(results.len(), 2);
    for order in &results {
        assert_eq!(order.status, "shipped");
        assert!(order.amount > 50);
    }
}

#[tokio::test]
async fn test_query_lsi() {
    use aymond::{Aymond, prelude::*, shim::futures::StreamExt};

    #[aymond(item, table)]
    struct Order {
        #[aymond(hash_key)]
        customer_id: String,

        #[aymond(sort_key)]
        order_id: String,

        #[aymond(gsi("by-status", hash_key))]
        status: String,

        #[aymond(gsi("by-status", sort_key))]
        #[aymond(lsi("by-amount"))]
        amount: i64,
    }

    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = OrderTable::new(&aymond, "lsi-query-test");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    // Insert test data for customer c1
    table
        .put()
        .item(Order {
            customer_id: "c1".into(),
            order_id: "o1".into(),
            status: "shipped".into(),
            amount: 100,
        })
        .send()
        .await
        .expect("Failed to put o1");

    table
        .put()
        .item(Order {
            customer_id: "c1".into(),
            order_id: "o2".into(),
            status: "pending".into(),
            amount: 50,
        })
        .send()
        .await
        .expect("Failed to put o2");

    table
        .put()
        .item(Order {
            customer_id: "c1".into(),
            order_id: "o3".into(),
            status: "shipped".into(),
            amount: 200,
        })
        .send()
        .await
        .expect("Failed to put o3");

    // Query LSI by-amount: customer c1, amount > 75
    let results: Vec<Order> = table
        .query_by_amount()
        .customer_id("c1")
        .amount_gt(75)
        .send()
        .await
        .map(|e| e.ok().unwrap())
        .collect()
        .await;

    assert_eq!(results.len(), 2);
    for order in &results {
        assert_eq!(order.customer_id, "c1");
        assert!(order.amount > 75);
    }
}
