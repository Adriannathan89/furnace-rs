//! Declarative infrastructure preserves native overrides through diamond imports.
#![cfg(feature = "sea-orm-postgres")]
use furnace_rs::{Logger, LoggerCauldron, LoggerService};
use furnace_rs_core::{ConfigBuilder, Furnace, MapSource, burner, cauldron};
use furnace_rs_persistence::sea_orm::{DatabaseCauldron, DatabaseConnection};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
#[burner]
struct Consumer {
    #[inject]
    database: DatabaseConnection,
    logger: Logger,
}
#[cauldron(imports=[DatabaseCauldron, LoggerCauldron])]
struct Left;
#[cauldron(imports=[DatabaseCauldron, LoggerCauldron])]
struct Right;
#[cauldron(root, imports=[Left, Right], providers=[Consumer])]
struct App;
struct RecordingLogger(Arc<AtomicUsize>);
impl LoggerService for RecordingLogger {
    fn log(&self, _: furnace_rs::LogLevel, _: &str) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
#[tokio::test]
async fn supplied_native_values_retain_one_owner_and_the_selected_logger() {
    let records = Arc::new(AtomicUsize::new(0));
    let database = sea_orm::MockDatabase::new(sea_orm::DbBackend::Postgres).into_connection();
    let config = ConfigBuilder::new()
        .source(MapSource::new(
            "test",
            [("persistence.seaorm.url", "postgres://localhost/test")],
        ))
        .build()
        .unwrap();
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<App>().unwrap();
    builder.provide(database).unwrap();
    builder
        .provide(Logger::new(RecordingLogger(records.clone())))
        .unwrap();
    let analysis = builder.analyze();
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics());
    let graph = analysis.cauldron_graph().unwrap();
    assert_eq!(
        graph
            .owner_of(std::any::TypeId::of::<DatabaseConnection>())
            .unwrap()
            .type_id(),
        std::any::TypeId::of::<DatabaseCauldron>()
    );
    let mut application = builder.build().await.unwrap();
    let consumer = application.context().resolve::<Consumer>().unwrap();
    assert_eq!(
        consumer.database.get_database_backend(),
        sea_orm::DbBackend::Postgres
    );
    consumer.logger.info("selected custom logger");
    assert_eq!(records.load(Ordering::SeqCst), 1);
    application.start().await.unwrap();
    application.shutdown().await.unwrap();
}
