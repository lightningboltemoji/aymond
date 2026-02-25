#[tokio::test]
async fn test_version_optimistic_locking() {
    use aymond::{Aymond, prelude::*};

    #[aymond(item, table)]
    struct Counter {
        #[aymond(hash_key)]
        id: String,
        count: i32,
        #[aymond(attribute(version))]
        ver: i64,
    }

    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = CounterTable::new(&aymond, "version_optimistic");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    // Initial insert via raw SDK (bypasses version check)
    let item_map: ::std::collections::HashMap<
        String,
        aymond::shim::aws_sdk_dynamodb::types::AttributeValue,
    > = Counter {
        id: "c1".to_string(),
        count: 0,
        ver: 1,
    }
    .into();
    aymond
        .client.put_item()
        .table_name("version_optimistic")
        .set_item(Some(item_map))
        .send()
        .await
        .expect("Initial put should succeed");

    // Put with correct version (ver=1 matches DB) — should succeed
    let c = Counter {
        id: "c1".to_string(),
        count: 1,
        ver: 1,
    };
    table
        .put()
        .item(c)
        .send()
        .await
        .expect("Put with correct version should succeed");

    // Now DB has ver=1 still (version doesn't auto-increment).
    // Overwrite with ver=2 directly so we can test staleness.
    let item_map: ::std::collections::HashMap<
        String,
        aymond::shim::aws_sdk_dynamodb::types::AttributeValue,
    > = Counter {
        id: "c1".to_string(),
        count: 10,
        ver: 2,
    }
    .into();
    aymond
        .client.put_item()
        .table_name("version_optimistic")
        .set_item(Some(item_map))
        .send()
        .await
        .expect("Direct put should succeed");

    // Put with stale ver=1 (DB has ver=2) — should fail
    let c = Counter {
        id: "c1".to_string(),
        count: 99,
        ver: 1,
    };
    let result = table.put().item(c).send().await;
    assert!(
        result.is_err(),
        "Put with stale version should fail (ConditionalCheckFailed)"
    );

    // Put with correct ver=2 — should succeed
    let c = Counter {
        id: "c1".to_string(),
        count: 11,
        ver: 2,
    };
    table
        .put()
        .item(c)
        .send()
        .await
        .expect("Put with correct version should succeed");
}

#[tokio::test]
async fn test_version_with_custom_name() {
    use aymond::{Aymond, prelude::*};

    #[aymond(item, table)]
    struct Widget {
        #[aymond(hash_key)]
        id: String,
        name: String,
        #[aymond(attribute(name = "v", version))]
        ver: i32,
    }

    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = WidgetTable::new(&aymond, "version_custom_name");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    // Initial insert via raw SDK
    let item_map: ::std::collections::HashMap<
        String,
        aymond::shim::aws_sdk_dynamodb::types::AttributeValue,
    > = Widget {
        id: "w1".to_string(),
        name: "Gear".to_string(),
        ver: 1,
    }
    .into();
    aymond
        .client.put_item()
        .table_name("version_custom_name")
        .set_item(Some(item_map))
        .send()
        .await
        .expect("Initial put should succeed");

    // Put with correct version
    let w = Widget {
        id: "w1".to_string(),
        name: "Gear v2".to_string(),
        ver: 1,
    };
    table
        .put()
        .item(w)
        .send()
        .await
        .expect("Put with correct version should succeed");

    // Verify the custom DDB name "v" is used
    let res = table
        .get()
        .id("w1")
        .raw(|r| r)
        .await
        .expect("Get should succeed");
    let item = res.item().expect("Item should exist");
    assert!(item.contains_key("v"), "Should use custom DDB name 'v'");
}

#[tokio::test]
async fn test_version_disable_versioning() {
    use aymond::{Aymond, prelude::*};

    #[aymond(item, table)]
    struct Counter {
        #[aymond(hash_key)]
        id: String,
        count: i32,
        #[aymond(attribute(version))]
        ver: i64,
    }

    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = CounterTable::new(&aymond, "version_disable");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    // Insert ver=5 directly
    let item_map: ::std::collections::HashMap<
        String,
        aymond::shim::aws_sdk_dynamodb::types::AttributeValue,
    > = Counter {
        id: "c1".to_string(),
        count: 0,
        ver: 5,
    }
    .into();
    aymond
        .client.put_item()
        .table_name("version_disable")
        .set_item(Some(item_map))
        .send()
        .await
        .expect("Initial put should succeed");

    // Put with stale ver=1 but disable_versioning — should succeed
    let c = Counter {
        id: "c1".to_string(),
        count: 99,
        ver: 1,
    };
    table
        .put()
        .item(c)
        .condition(|c| {
            c.disable_versioning();
            c.id().eq("c1")
        })
        .send()
        .await
        .expect("Put with disable_versioning should bypass version check");
}

#[tokio::test]
async fn test_version_delete_via_item() {
    use aymond::{Aymond, prelude::*};

    #[aymond(item, table)]
    struct Counter {
        #[aymond(hash_key)]
        id: String,
        count: i32,
        #[aymond(attribute(version))]
        ver: i64,
    }

    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = CounterTable::new(&aymond, "version_delete_item");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    // Insert ver=3 directly
    let item_map: ::std::collections::HashMap<
        String,
        aymond::shim::aws_sdk_dynamodb::types::AttributeValue,
    > = Counter {
        id: "c1".to_string(),
        count: 10,
        ver: 3,
    }
    .into();
    aymond
        .client.put_item()
        .table_name("version_delete_item")
        .set_item(Some(item_map))
        .send()
        .await
        .expect("Initial put should succeed");

    // Delete with stale version via .item() — should fail
    let c = Counter {
        id: "c1".to_string(),
        count: 10,
        ver: 1,
    };
    let result = table.delete_item().item(c).send().await;
    assert!(result.is_err(), "Delete with stale version should fail");

    // Verify item still exists
    let get = table.get().id("c1").send().await.unwrap();
    assert!(get.is_some(), "Item should still exist after failed delete");

    // Delete with correct version via .item() — should succeed
    let c = Counter {
        id: "c1".to_string(),
        count: 10,
        ver: 3,
    };
    table
        .delete_item()
        .item(c)
        .send()
        .await
        .expect("Delete with correct version should succeed");

    // Verify item is gone
    let get = table.get().id("c1").send().await.unwrap();
    assert!(get.is_none(), "Item should be deleted");
}

#[tokio::test]
async fn test_version_delete_explicit_keys_no_version_check() {
    use aymond::{Aymond, prelude::*};

    #[aymond(item, table)]
    struct Counter {
        #[aymond(hash_key)]
        id: String,
        count: i32,
        #[aymond(attribute(version))]
        ver: i64,
    }

    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = CounterTable::new(&aymond, "version_delete_explicit");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    // Insert ver=5 directly
    let item_map: ::std::collections::HashMap<
        String,
        aymond::shim::aws_sdk_dynamodb::types::AttributeValue,
    > = Counter {
        id: "c1".to_string(),
        count: 10,
        ver: 5,
    }
    .into();
    aymond
        .client.put_item()
        .table_name("version_delete_explicit")
        .set_item(Some(item_map))
        .send()
        .await
        .expect("Initial put should succeed");

    // Delete with explicit key (no .item()), version_value stays None.
    // No version check applied even though versioning=true.
    table
        .delete_item()
        .id("c1")
        .send()
        .await
        .expect("Delete with explicit key should succeed without version check");

    // Verify item is gone
    let get = table.get().id("c1").send().await.unwrap();
    assert!(get.is_none(), "Item should be deleted");
}

#[tokio::test]
async fn test_version_with_user_condition_combined() {
    use aymond::{Aymond, prelude::*};

    #[aymond(item, table)]
    struct Counter {
        #[aymond(hash_key)]
        id: String,
        count: i32,
        #[aymond(attribute(version))]
        ver: i64,
    }

    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = CounterTable::new(&aymond, "version_combined_cond");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    // Insert
    let item_map: ::std::collections::HashMap<
        String,
        aymond::shim::aws_sdk_dynamodb::types::AttributeValue,
    > = Counter {
        id: "c1".to_string(),
        count: 10,
        ver: 1,
    }
    .into();
    aymond
        .client.put_item()
        .table_name("version_combined_cond")
        .set_item(Some(item_map))
        .send()
        .await
        .expect("Initial put should succeed");

    // Put with correct version AND true user condition — should succeed
    let c = Counter {
        id: "c1".to_string(),
        count: 20,
        ver: 1,
    };
    table
        .put()
        .item(c)
        .condition(|c| c.count().eq(10i32))
        .send()
        .await
        .expect("Put with correct version and true user condition should succeed");

    // Verify update happened
    let get = table.get().id("c1").send().await.unwrap().unwrap();
    assert_eq!(get.count, 20);
}
