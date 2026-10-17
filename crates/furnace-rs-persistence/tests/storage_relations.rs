//! Generated loaders preserve relation identities and parent page boundaries.
#![cfg(feature = "sea-orm-storage")]
#[path = "support/storage_relations.rs"]
mod entities;
use entities::{comment, post, tag, user};
use furnace_rs_core::{Injector, storage};
use furnace_rs_persistence::storage::StorageErrorKind;
use sea_orm::{ColumnTrait, DatabaseConnection, DbBackend, MockDatabase};
#[storage(entity=post::Entity,relations(author=post::Relation::Author,editor=post::Relation::Editor,comments=comment::Entity,tags=tag::Entity))]
struct Repo {
    #[inject]
    database: DatabaseConnection,
}
fn row(id: i32) -> post::Model {
    post::Model {
        id,
        title: format!("post{id}"),
        author_id: 1,
        editor_id: 2,
    }
}

#[tokio::test]
async fn second_page_uses_requested_limit_and_not_lookahead_size() {
    use sea_orm::Transaction;
    let db = MockDatabase::new(DbBackend::Postgres)
        .append_query_results([vec![row(21)]])
        .into_connection();
    let repo = Repo::inject((db,)).await.unwrap();
    let page = repo
        .entity()
        .query()
        .order_by_asc(post::Column::Id)
        .paginate(2, 20)
        .await
        .unwrap();
    assert_eq!(page.items[0].id, 21);
    assert_eq!(
        repo.database.clone().into_transaction_log(),
        vec![Transaction::from_sql_and_values(
            DbBackend::Postgres,
            r#"SELECT "storage_loaded_posts"."id", "storage_loaded_posts"."title", "storage_loaded_posts"."author_id", "storage_loaded_posts"."editor_id" FROM "storage_loaded_posts" ORDER BY "storage_loaded_posts"."id" ASC LIMIT $1 OFFSET $2"#,
            [20u64.into(), 20u64.into()]
        )]
    );
}

#[tokio::test]
async fn many_to_many_uses_native_batch_mapping() {
    use std::collections::BTreeMap;
    let joined: Vec<BTreeMap<String, sea_orm::Value>> =
        [(1, 21, "rust"), (2, 21, "web"), (1, 22, "rust")]
            .into_iter()
            .map(|(id, post_id, name)| {
                BTreeMap::from([
                    ("id".into(), id.into()),
                    ("name".into(), name.into()),
                    ("post_id".into(), post_id.into()),
                ])
            })
            .collect();
    let db = MockDatabase::new(DbBackend::Postgres)
        .append_query_results([vec![row(21), row(22)]])
        .append_query_results([joined])
        .into_connection();
    let repo = Repo::inject((db,)).await.unwrap();
    let page = repo
        .entity()
        .query()
        .order_by_asc(post::Column::Id)
        .with_tags()
        .paginate(1, 2)
        .await
        .unwrap_or_else(|error| {
            panic!(
                "native loader failure: {:?}",
                std::error::Error::source(&error)
            )
        });
    assert_eq!(
        page.items[0]
            .tags
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>(),
        vec!["rust", "web"]
    );
    assert_eq!(page.items[1].tags.len(), 1);
    let logs = repo.database.clone().into_transaction_log();
    assert_eq!(logs.len(), 2);
    assert!(format!("{logs:?}").contains("INNER JOIN"));
}

#[tokio::test]
async fn no_selected_relations_remain_unloaded_and_basic_crud_returns_model() {
    let db = MockDatabase::new(DbBackend::Postgres)
        .append_query_results([vec![row(1)], vec![row(1)]])
        .into_connection();
    let repo = Repo::inject((db,)).await.unwrap();
    let basic: Option<post::Model> = repo.entity().find_by_id(1).await.unwrap();
    assert!(basic.is_some());
    let query = repo.entity().query().order_by_desc(post::Column::Id);
    let page = query.paginate(1, 20).await.unwrap();
    assert_eq!(page.items[0].id, 1);
    assert!(page.items[0].author.is_unloaded());
    assert!(page.items[0].comments.is_unloaded());
    assert_eq!(repo.database.clone().into_transaction_log().len(), 2);
}

#[tokio::test]
async fn author_editor_and_repeated_selection_use_correct_native_paths() {
    let db = MockDatabase::new(DbBackend::Postgres)
        .append_query_results([vec![row(1)]])
        .append_query_results([
            vec![user::Model {
                id: 1,
                name: "author".into(),
            }],
            vec![user::Model {
                id: 2,
                name: "editor".into(),
            }],
        ])
        .into_connection();
    let repo = Repo::inject((db,)).await.unwrap();
    let item = repo
        .entity()
        .query()
        .filter(post::Column::Id.eq(1))
        .with_author()
        .with_author()
        .with_editor()
        .one()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(item.author.as_ref().unwrap().name, "author");
    assert_eq!(item.editor.as_ref().unwrap().name, "editor");
    let logs = repo.database.clone().into_transaction_log();
    assert_eq!(logs.len(), 3);
}

#[tokio::test]
async fn collection_loading_preserves_requested_roots_without_lookahead() {
    let db = MockDatabase::new(DbBackend::Postgres)
        .append_query_results([vec![row(21), row(22)]])
        .append_query_results([vec![
            comment::Model {
                id: 1,
                post_id: 21,
                body: "a".into(),
            },
            comment::Model {
                id: 2,
                post_id: 21,
                body: "b".into(),
            },
            comment::Model {
                id: 3,
                post_id: 21,
                body: "c".into(),
            },
            comment::Model {
                id: 4,
                post_id: 22,
                body: "d".into(),
            },
        ]])
        .into_connection();
    let repo = Repo::inject((db,)).await.unwrap();
    let page = repo
        .entity()
        .query()
        .order_by_asc(post::Column::Id)
        .with_comments()
        .paginate(11, 2)
        .await
        .unwrap();
    assert_eq!(
        page.items.iter().map(|p| p.id).collect::<Vec<_>>(),
        vec![21, 22]
    );
    assert_eq!(page.items[0].comments.len(), 3);
    assert_eq!(page.items[1].comments.len(), 1);
    let logs = repo.database.clone().into_transaction_log();
    assert_eq!(logs.len(), 2);
    let sql = format!("{logs:?}");
    assert!(!sql.contains("COUNT"));
    assert!(sql.contains("LIMIT"));
}

#[tokio::test]
async fn validation_precedes_all_parent_and_relation_queries() {
    let db = MockDatabase::new(DbBackend::Postgres).into_connection();
    let repo = Repo::inject((db,)).await.unwrap();
    let error = repo
        .entity()
        .query()
        .with_comments()
        .paginate(1, 20)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), StorageErrorKind::MissingOrder);
    let error = repo
        .entity()
        .query()
        .with_author()
        .paginate(0, 20)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), StorageErrorKind::InvalidPagination);
    assert!(repo.database.clone().into_transaction_log().is_empty());
}
