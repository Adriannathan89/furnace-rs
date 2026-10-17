//! Generated entity access borrows the selected injected connection.
#![cfg(feature = "sea-orm-storage")]
#[path = "support/storage_entities.rs"]
mod entities;
use entities::post;
use furnace_rs_core::{Injector, storage};
use sea_orm::{DatabaseConnection, DbBackend, MockDatabase};
type Db = DatabaseConnection;
#[storage(entity=post::Entity)]
struct Inferred {
    #[inject]
    database: DatabaseConnection,
}
#[storage(entity=post::Entity,connection=primary)]
struct Explicit {
    #[inject]
    primary: Db,
    #[inject]
    replica: DatabaseConnection,
}

#[tokio::test]
async fn generated_accessor_uses_the_selected_connection_and_keeps_query_lifetime() {
    let row = post::Model {
        id: 1,
        title: "selected".into(),
        published: true,
    };
    let primary = MockDatabase::new(DbBackend::Postgres)
        .append_query_results([vec![row.clone()]])
        .into_connection();
    let replica = MockDatabase::new(DbBackend::Postgres).into_connection();
    let storage = Explicit::inject((primary, replica)).await.unwrap();
    assert_eq!(
        storage.entity().find_by_id(1).await.unwrap(),
        Some(row.clone())
    );
    assert!(storage.replica.clone().into_transaction_log().is_empty());
    let db = MockDatabase::new(DbBackend::Postgres)
        .append_query_results([vec![row.clone()]])
        .into_connection();
    let storage = Inferred::inject((db,)).await.unwrap();
    let query = storage.entity().query().order_by_asc(post::Column::Id);
    assert_eq!(query.paginate(1, 20).await.unwrap().items, vec![row]);
}
