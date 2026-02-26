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

    // Initial insert using version-zero (safe creation)
    // ver=0 → attribute_not_exists condition, DB gets ver=1
    let c = Counter {
        id: "c1".to_string(),
        count: 0,
        ver: 0,
    };
    table
        .put()
        .item(c)
        .send()
        .await
        .expect("Version-zero initial put should succeed");

    // Verify DB has ver=1 after auto-increment
    let get = table.get().id("c1").send().await.unwrap().unwrap();
    assert_eq!(get.ver, 1);

    // Put with correct ver=1 (matches DB) — should succeed, DB gets ver=2
    let c = Counter {
        id: "c1".to_string(),
        count: 5,
        ver: 1,
    };
    table
        .put()
        .item(c)
        .send()
        .await
        .expect("Put with correct version should succeed");

    // Verify DB has ver=2
    let get = table.get().id("c1").send().await.unwrap().unwrap();
    assert_eq!(get.ver, 2);
    assert_eq!(get.count, 5);

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

    // Put with correct ver=2 — should succeed, DB gets ver=3
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

    // Verify DB has ver=3
    let get = table.get().id("c1").send().await.unwrap().unwrap();
    assert_eq!(get.ver, 3);
    assert_eq!(get.count, 11);
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

    // Initial insert using version-zero, DB gets ver=1
    let w = Widget {
        id: "w1".to_string(),
        name: "Gear".to_string(),
        ver: 0,
    };
    table
        .put()
        .item(w)
        .send()
        .await
        .expect("Version-zero initial put should succeed");

    // Put with correct ver=1, DB gets ver=2
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

    // Insert ver=0, DB gets ver=1
    let c = Counter {
        id: "c1".to_string(),
        count: 0,
        ver: 0,
    };
    table
        .put()
        .item(c)
        .send()
        .await
        .expect("Initial put should succeed");

    // Put with stale ver=99 but disable_versioning — should succeed
    // No auto-increment since versioning is disabled, DB gets ver=99
    let c = Counter {
        id: "c1".to_string(),
        count: 99,
        ver: 99,
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

    // Insert ver=0, DB gets ver=1. Then put ver=1, DB gets ver=2. Then ver=2, DB gets ver=3.
    let c = Counter {
        id: "c1".to_string(),
        count: 10,
        ver: 0,
    };
    table.put().item(c).send().await.expect("put should succeed");
    let c = Counter {
        id: "c1".to_string(),
        count: 10,
        ver: 1,
    };
    table.put().item(c).send().await.expect("put should succeed");
    let c = Counter {
        id: "c1".to_string(),
        count: 10,
        ver: 2,
    };
    table.put().item(c).send().await.expect("put should succeed");

    // DB now has ver=3

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

    // Insert ver=0, DB gets ver=1
    let c = Counter {
        id: "c1".to_string(),
        count: 10,
        ver: 0,
    };
    table
        .put()
        .item(c)
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

    // Insert ver=0, DB gets ver=1
    let c = Counter {
        id: "c1".to_string(),
        count: 10,
        ver: 0,
    };
    table
        .put()
        .item(c)
        .send()
        .await
        .expect("Initial put should succeed");

    // Put with correct version AND true user condition — should succeed
    // ver=1 matches DB, condition count=10 also matches, DB gets ver=2
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
    assert_eq!(get.ver, 2);
}

#[tokio::test]
async fn test_version_zero_initial_creation() {
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
    let table = CounterTable::new(&aymond, "version_zero_create");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    // Version-zero put on empty table — should succeed (generates attribute_not_exists)
    // DB gets ver=1 after auto-increment
    let c = Counter {
        id: "c1".to_string(),
        count: 0,
        ver: 0,
    };
    table
        .put()
        .item(c)
        .send()
        .await
        .expect("Version-zero put on empty table should succeed");

    // Verify item was created with ver=1
    let get = table.get().id("c1").send().await.unwrap().unwrap();
    assert_eq!(get.ver, 1);

    // Version-zero put again — should fail (item already exists)
    let c = Counter {
        id: "c1".to_string(),
        count: 1,
        ver: 0,
    };
    let result = table.put().item(c).send().await;
    assert!(
        result.is_err(),
        "Version-zero put should fail when item already exists"
    );
}

#[tokio::test]
async fn test_must_not_exist_condition() {
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
    let table = CounterTable::new(&aymond, "version_must_not_exist");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    // Use must_not_exist() explicitly for safe creation
    // ver=5 → auto-incremented to 6, but must_not_exist overrides version check
    let c = Counter {
        id: "c1".to_string(),
        count: 0,
        ver: 5,
    };
    table
        .put()
        .item(c)
        .condition(|c| {
            c.must_not_exist();
        })
        .send()
        .await
        .expect("must_not_exist on empty table should succeed");

    // Verify ver was auto-incremented to 6
    let get = table.get().id("c1").send().await.unwrap().unwrap();
    assert_eq!(get.ver, 6);

    // Try again — should fail because item exists
    let c = Counter {
        id: "c1".to_string(),
        count: 1,
        ver: 5,
    };
    let result = table
        .put()
        .item(c)
        .condition(|c| {
            c.must_not_exist();
        })
        .send()
        .await;
    assert!(
        result.is_err(),
        "must_not_exist should fail when item already exists"
    );
}

#[tokio::test]
async fn test_must_exist_condition() {
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
    let table = CounterTable::new(&aymond, "version_must_exist");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    // must_exist on empty table — should fail
    let c = Counter {
        id: "c1".to_string(),
        count: 0,
        ver: 5,
    };
    let result = table
        .put()
        .item(c)
        .condition(|c| {
            c.must_exist();
        })
        .send()
        .await;
    assert!(
        result.is_err(),
        "must_exist should fail when item doesn't exist"
    );

    // Insert item first (ver=0 → DB gets ver=1)
    let c = Counter {
        id: "c1".to_string(),
        count: 0,
        ver: 0,
    };
    table
        .put()
        .item(c)
        .send()
        .await
        .expect("Version-zero initial put should succeed");

    // must_exist on existing item — should succeed (overrides version check)
    // ver=999 → auto-incremented to 1000, but must_exist overrides version check
    let c = Counter {
        id: "c1".to_string(),
        count: 10,
        ver: 999,
    };
    table
        .put()
        .item(c)
        .condition(|c| {
            c.must_exist();
        })
        .send()
        .await
        .expect("must_exist should succeed when item exists");

    // Verify update happened with auto-incremented ver
    let get = table.get().id("c1").send().await.unwrap().unwrap();
    assert_eq!(get.count, 10);
    assert_eq!(get.ver, 1000);
}

#[tokio::test]
async fn test_must_not_exist_on_non_versioned_item() {
    use aymond::{Aymond, prelude::*};

    #[aymond(item, table)]
    struct SimpleItem {
        #[aymond(hash_key)]
        id: String,
        value: String,
    }

    let aymond = Aymond::new_with_local_config("http://localhost:8000", "us-west-2");
    let table = SimpleItemTable::new(&aymond, "non_versioned_existence");
    table.delete(false).await.expect("Failed to delete");
    table.create(false).await.expect("Failed to create");

    // must_not_exist on empty table — should succeed
    let item = SimpleItem {
        id: "s1".to_string(),
        value: "hello".to_string(),
    };
    table
        .put()
        .item(item)
        .condition(|c| {
            c.must_not_exist();
        })
        .send()
        .await
        .expect("must_not_exist on empty table should succeed");

    // must_not_exist again — should fail
    let item = SimpleItem {
        id: "s1".to_string(),
        value: "world".to_string(),
    };
    let result = table
        .put()
        .item(item)
        .condition(|c| {
            c.must_not_exist();
        })
        .send()
        .await;
    assert!(
        result.is_err(),
        "must_not_exist should fail when item already exists"
    );
}
