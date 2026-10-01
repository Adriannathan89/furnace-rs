//! Function-level fixture macro and Cargo test discovery.
#![allow(missing_docs)]
use furnace_rs_testing::sea_orm::{DatabaseConnection, DbBackend, MockDatabase};
#[furnace_rs::storage]
struct Repository {
    database: DatabaseConnection,
}
#[furnace_rs::burner]
struct Service {
    repository: Repository,
}
#[furnace_rs::test]
async fn service_fixture_runs() {
    test_fixture()
        .mock_database(MockDatabase::new(DbBackend::Sqlite))
        .subject::<Service>()
        .run(|context| async move {
            use furnace_rs_testing::sea_orm::ConnectionTrait;
            assert!(
                context
                    .resolve::<Service>()
                    .unwrap()
                    .repository
                    .database
                    .is_mock_connection()
            );
        })
        .await
        .unwrap();
}
#[cfg(feature = "http")]
#[furnace_rs::routes]
trait Routes {
    #[get("/fixture")]
    async fn fixture(&self) -> &'static str;
}
#[cfg(feature = "http")]
#[furnace_rs::controller(routes = [Routes])]
struct Controller;
#[cfg(feature = "http")]
impl Routes for Controller {
    async fn fixture(&self) -> &'static str {
        "fixture"
    }
}
#[cfg(feature = "http")]
#[furnace_rs::test]
async fn controller_fixture_runs() -> furnace_rs_testing::TestResult<()> {
    test_fixture()
        .controller::<Controller>()
        .run(|client| async move {
            client
                .get("/fixture")
                .send()
                .await
                .unwrap()
                .assert_text("fixture");
        })
        .await
}
#[test]
fn test_attribute_compile_fail() {
    trybuild::TestCases::new().compile_fail("tests/ui-test-attribute/*.rs");
}
