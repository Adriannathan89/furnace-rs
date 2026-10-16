//! PostgreSQL acceptance for generated storage and native relation loading.
#![cfg(all(feature = "sea-orm-storage", feature = "sea-orm-postgres"))]
#[path = "support/storage_entities.rs"]
mod basic;
#[path = "support/storage_relations.rs"]
mod entities;
use entities::{comment, post, post_tag, tag, user};
use furnace_rs_core::{Injector, storage};
use furnace_rs_persistence::storage::{EntityStore, StorageErrorKind};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, ConnectOptions, ConnectionTrait, Database, DatabaseConnection,
    DbBackend, EntityTrait, Schema, TransactionTrait,
};

#[storage(entity=post::Entity,relations(author=post::Relation::Author,editor=post::Relation::Editor,comments=comment::Entity,tags=tag::Entity))]
struct Repo {
    #[inject]
    database: DatabaseConnection,
}

async fn exercise(database: DatabaseConnection) {
    let schema = Schema::new(DbBackend::Postgres);
    database
        .execute(&schema.create_table_from_entity(user::Entity))
        .await
        .unwrap();
    database
        .execute(&schema.create_table_from_entity(post::Entity))
        .await
        .unwrap();
    database
        .execute(&schema.create_table_from_entity(comment::Entity))
        .await
        .unwrap();
    database
        .execute(&schema.create_table_from_entity(tag::Entity))
        .await
        .unwrap();
    database
        .execute(&schema.create_table_from_entity(post_tag::Entity))
        .await
        .unwrap();
    database
        .execute(&schema.create_table_from_entity(basic::composite::Entity))
        .await
        .unwrap();
    database
        .execute(&schema.create_table_from_entity(basic::hooked::Entity))
        .await
        .unwrap();

    let repo = Repo::inject((database,)).await.unwrap();
    let users = EntityStore::<user::Entity>::new(&repo.database);
    for (id, name) in [(1, "author"), (2, "editor")] {
        users
            .insert(user::ActiveModel {
                id: Set(id),
                name: Set(name.into()),
            })
            .await
            .unwrap();
    }
    for id in 1..=25 {
        repo.entity()
            .insert(post::ActiveModel {
                id: Set(id),
                title: Set("same".into()),
                author_id: Set(1),
                editor_id: Set(2),
            })
            .await
            .unwrap();
    }
    for (id, post_id) in [(1, 21), (2, 21), (3, 21), (4, 22)] {
        EntityStore::<comment::Entity>::new(&repo.database)
            .insert(comment::ActiveModel {
                id: Set(id),
                post_id: Set(post_id),
                body: Set("comment".into()),
            })
            .await
            .unwrap();
    }
    for (id, name) in [(1, "rust"), (2, "web")] {
        EntityStore::<tag::Entity>::new(&repo.database)
            .insert(tag::ActiveModel {
                id: Set(id),
                name: Set(name.into()),
            })
            .await
            .unwrap();
    }
    for (post_id, tag_id) in [(21, 1), (21, 2), (22, 1)] {
        EntityStore::<post_tag::Entity>::new(&repo.database)
            .insert(post_tag::ActiveModel {
                post_id: Set(post_id),
                tag_id: Set(tag_id),
            })
            .await
            .unwrap();
    }

    let first = repo
        .entity()
        .query()
        .order_by_desc(post::Column::Title)
        .order_by_asc(post::Column::Id)
        .paginate(1, 20)
        .await
        .unwrap();
    assert_eq!(
        first.items.iter().map(|m| m.id).collect::<Vec<_>>(),
        (1..=20).collect::<Vec<_>>()
    );
    let page = repo
        .entity()
        .query()
        .order_by_asc(post::Column::Id)
        .with_author()
        .with_editor()
        .with_comments()
        .with_tags()
        .paginate(2, 20)
        .await
        .unwrap();
    assert_eq!(
        page.items.iter().map(|m| m.id).collect::<Vec<_>>(),
        vec![21, 22, 23, 24, 25]
    );
    assert_eq!(page.items[0].author.as_ref().unwrap().name, "author");
    assert_eq!(page.items[0].editor.as_ref().unwrap().name, "editor");
    assert_eq!(page.items[0].comments.len(), 3);
    assert_eq!(page.items[1].comments.len(), 1);
    assert_eq!(page.items[0].tags.len(), 2);
    assert_eq!(page.items[1].tags.len(), 1);
    assert_eq!((page.page_number, page.page_size), (2, 20));
    assert!(
        repo.entity()
            .query()
            .order_by_asc(post::Column::Id)
            .with_comments()
            .paginate(3, 20)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    assert_eq!(
        repo.entity()
            .query()
            .with_comments()
            .paginate(1, 20)
            .await
            .unwrap_err()
            .kind(),
        StorageErrorKind::MissingOrder
    );
    let filtered = repo
        .entity()
        .query()
        .filter(post::Column::Id.gte(21))
        .order_by_desc(post::Column::Id)
        .paginate(1, 2)
        .await
        .unwrap();
    assert_eq!(
        filtered.items.iter().map(|m| m.id).collect::<Vec<_>>(),
        vec![25, 24]
    );

    let composites = EntityStore::<basic::composite::Entity>::new(&repo.database);
    for tenant_id in 1..=2 {
        for id in 1..=3 {
            composites
                .insert(basic::composite::ActiveModel {
                    tenant_id: Set(tenant_id),
                    id: Set(id),
                    title: Set("same".into()),
                })
                .await
                .unwrap();
        }
    }
    let page = composites
        .query()
        .order_by_asc(basic::composite::Column::Title)
        .order_by_desc(basic::composite::Column::TenantId)
        .order_by_asc(basic::composite::Column::Id)
        .paginate(2, 2)
        .await
        .unwrap();
    assert_eq!(
        page.items
            .iter()
            .map(|m| (m.tenant_id, m.id))
            .collect::<Vec<_>>(),
        vec![(2, 3), (1, 1)]
    );
    assert!(composites.find_by_id((2, 3)).await.unwrap().is_some());
    assert!(composites.delete_by_id((2, 3)).await.unwrap());
    assert!(!composites.delete_by_id((2, 3)).await.unwrap());

    let updated = repo
        .entity()
        .update(post::ActiveModel {
            id: Set(24),
            title: Set("changed".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(updated.title, "changed");
    let error = repo
        .entity()
        .update(post::ActiveModel {
            id: Set(999),
            title: Set("missing".into()),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert_eq!(error.kind(), StorageErrorKind::NotFound);
    assert!(repo.entity().delete_by_id(25).await.unwrap());
    assert!(!repo.entity().delete_by_id(25).await.unwrap());
    assert!(repo.entity().find_by_id(999).await.unwrap().is_none());

    let hooks = EntityStore::<basic::hooked::Entity>::new(&repo.database);
    let saved = hooks
        .insert(basic::hooked::ActiveModel {
            title: Set("input".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(saved.title, "before");
    hooks
        .update(basic::hooked::ActiveModel {
            id: Set(saved.id),
            title: Set("new".into()),
        })
        .await
        .unwrap();
    assert_eq!(
        basic::hooked::BEFORE.load(std::sync::atomic::Ordering::SeqCst),
        2
    );
    assert_eq!(
        basic::hooked::AFTER.load(std::sync::atomic::Ordering::SeqCst),
        2
    );

    // The application owns this transaction and passes its handle directly.
    let transaction = repo.database.begin().await.unwrap();
    post::Entity::insert(post::ActiveModel {
        id: Set(100),
        title: Set("rolled back".into()),
        author_id: Set(1),
        editor_id: Set(2),
    })
    .exec(&transaction)
    .await
    .unwrap();
    transaction.rollback().await.unwrap();
    assert!(repo.entity().find_by_id(100).await.unwrap().is_none());
    repo.database.clone().close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires PostgreSQL through FURNACE_TEST_DATABASE_URL"]
async fn native_storage_crud_relations_ordering_and_pagination() {
    let url = std::env::var("FURNACE_TEST_DATABASE_URL").expect("test database URL is required");
    let control = Database::connect(url.clone()).await.unwrap();
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let schema = format!("furnace_storage_{}_{suffix}", std::process::id());
    control
        .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
        .await
        .unwrap();
    let mut options = ConnectOptions::new(url);
    options
        .max_connections(1)
        .set_schema_search_path(schema.clone());
    let database = Database::connect(options).await.unwrap();
    let result = tokio::spawn(exercise(database)).await;
    control
        .execute_unprepared(&format!("DROP SCHEMA {schema} CASCADE"))
        .await
        .unwrap();
    control.close().await.unwrap();
    result.expect("native storage acceptance task failed");
}
