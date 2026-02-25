/// Compile-only test: verifies that GSI and LSI annotations are accepted by the macro.
/// No DynamoDB operations are performed; we just confirm the struct derives without error.
#[test]
fn gsi_lsi_annotations_compile() {
    use aymond::prelude::*;

    #[aymond(item, table)]
    struct Car {
        #[aymond(hash_key)]
        make: String,

        #[aymond(sort_key)]
        #[aymond(global_secondary_index("by-year", hash_key))]
        model: String,

        #[aymond(gsi("by-year", sort_key))]
        #[aymond(lsi("by-color"))]
        year: i32,

        color: Option<String>,
    }

    // Confirm the derived struct and table type exist
    let _ = std::marker::PhantomData::<CarTable>;
    let _ = std::marker::PhantomData::<Car>;
}

#[tokio::test]
async fn test_create_with_secondary_indexes() {
    use aymond::{HighLevelClient, prelude::*};

    #[aymond(item, table)]
    struct Order {
        #[aymond(hash_key)]
        customer_id: String,

        #[aymond(sort_key)]
        order_id: String,

        #[aymond(gsi("by-status", hash_key))]
        #[aymond(lsi("by-amount"))]
        status: String,

        #[aymond(gsi("by-status", sort_key))]
        amount: i64,
    }

    let client = HighLevelClient::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = OrderTable::new(&client, "secondary-indexes-test");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create table with GSI and LSI");
}
