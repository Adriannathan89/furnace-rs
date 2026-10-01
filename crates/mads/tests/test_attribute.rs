//! Function-level fixture macro and Cargo test discovery.
#![allow(missing_docs)]
use mads_testing::sea_orm::{DatabaseConnection, DbBackend, MockDatabase};
#[mads::repository]
struct Repository {
    database: DatabaseConnection,
}
#[mads::service]
struct Service {
    repository: Repository,
}
#[mads::test]
async fn service_fixture_runs() {
    test_fixture()
        .mock_database(MockDatabase::new(DbBackend::Sqlite))
        .subject::<Service>()
        .run(|context| async move {
            use mads_testing::sea_orm::ConnectionTrait;
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
#[mads::routes]
trait Routes {
    #[get("/fixture")]
    async fn fixture(&self) -> &'static str;
}
#[cfg(feature = "http")]
#[mads::controller(routes = [Routes])]
struct Controller;
#[cfg(feature = "http")]
impl Routes for Controller {
    async fn fixture(&self) -> &'static str {
        "fixture"
    }
}
#[cfg(feature = "http")]
#[mads::test]
async fn controller_fixture_runs() -> mads_testing::TestResult<()> {
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
