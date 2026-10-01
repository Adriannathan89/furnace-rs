//! Subject fixtures and explicit scripted database supplies.
#![allow(missing_docs)]
use futures_util::FutureExt;
use mads_core::{
    ApplicationContext, Diagnostic, Error, LifecycleFuture, LifecycleHook, LifecycleResource,
    MADS010,
};
use mads_testing::{
    __private::test_fixture,
    TestError,
    sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, MockDatabase, Statement},
};
use std::{
    collections::BTreeMap,
    panic::AssertUnwindSafe,
    sync::{Arc, Mutex},
};

#[mads_core::repository]
struct Repository {
    db: DatabaseConnection,
}
impl Repository {
    async fn name(&self) -> String {
        self.db
            .query_one_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT name FROM users",
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "name")
            .unwrap()
    }
}
#[mads_core::service]
struct Service {
    repo: Repository,
}
#[mads_core::service]
struct Unrelated;
// Calling a registered production connector is a test failure.
#[mads_core::provider]
fn production_database() -> DatabaseConnection {
    panic!("production connector ran")
}

#[tokio::test]
async fn service_uses_mock_database_and_private_chain() {
    let row = BTreeMap::from([("name", sea_orm::Value::String(Some("Ada".to_owned())))]);
    let mock = MockDatabase::new(DbBackend::Sqlite).append_query_results([[row]]);
    test_fixture()
        .mock_database(mock)
        .subject::<Service>()
        .run(|app| async move {
            assert_eq!(app.resolve::<Service>().unwrap().repo.name().await, "Ada");
            assert!(app.resolve::<Unrelated>().is_err());
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn missing_mock_never_connects() {
    let error = test_fixture()
        .subject::<Service>()
        .run(|_| async { panic!("body ran") })
        .await
        .unwrap_err();
    assert!(matches!(error, TestError::MissingMockDatabase));
}
#[tokio::test]
async fn non_sqlite_mock_is_rejected() {
    for backend in [DbBackend::Postgres, DbBackend::MySql] {
        let error = test_fixture()
            .mock_database(MockDatabase::new(backend))
            .subject::<Service>()
            .run(|_| async {})
            .await
            .unwrap_err();
        assert!(matches!(error, TestError::UnsupportedMockBackend(_)));
    }
}
#[tokio::test]
async fn direct_database_supply_is_rejected() {
    let db = MockDatabase::new(DbBackend::Sqlite).into_connection();
    let error = test_fixture()
        .provide(db)
        .subject::<Service>()
        .run(|_| async {})
        .await
        .unwrap_err();
    assert!(matches!(error, TestError::DirectDatabaseSupply));
}
#[tokio::test]
async fn duplicate_supply_is_rejected_before_construction() {
    let error = test_fixture()
        .provide(1_u32)
        .provide(2_u32)
        .subject::<Service>()
        .run(|_| async {})
        .await
        .unwrap_err();
    assert!(matches!(error, TestError::DuplicateSupply(_)));
}
#[tokio::test]
async fn duplicate_mock_is_rejected() {
    let error = test_fixture()
        .mock_database(MockDatabase::new(DbBackend::Sqlite))
        .mock_database(MockDatabase::new(DbBackend::Sqlite))
        .subject::<Service>()
        .run(|_| async {})
        .await
        .unwrap_err();
    assert!(matches!(error, TestError::DuplicateSupply(_)));
}

#[derive(Clone, Default)]
struct Events(Arc<Mutex<Vec<&'static str>>>);
#[derive(Clone)]
struct Mode {
    fail_start: bool,
    fail_stop: bool,
}
struct Hook {
    events: Events,
    mode: Mode,
    name: &'static str,
}
impl LifecycleHook for Hook {
    fn name(&self) -> &str {
        self.name
    }
    fn start<'a>(&'a self, _: &'a ApplicationContext) -> LifecycleFuture<'a> {
        Box::pin(async move {
            self.events.0.lock().unwrap().push(if self.name == "first" {
                "start:first"
            } else {
                "start:last"
            });
            if self.mode.fail_start {
                Err(Error::new(Diagnostic::new(MADS010, "hook", "start failed")))
            } else {
                Ok(())
            }
        })
    }
    fn stop<'a>(&'a self, _: &'a ApplicationContext) -> LifecycleFuture<'a> {
        Box::pin(async move {
            tokio::task::yield_now().await;
            self.events.0.lock().unwrap().push(if self.name == "first" {
                "stop:first"
            } else {
                "stop:last"
            });
            if self.mode.fail_stop {
                Err(Error::new(Diagnostic::new(MADS010, "hook", "stop failed")))
            } else {
                Ok(())
            }
        })
    }
}
struct Managed;
#[mads_core::provider(lifecycle)]
async fn managed(events: Events, mode: Mode) -> LifecycleResource<Managed> {
    LifecycleResource::new(Managed)
        .with_infrastructure_hook(
            "test.first",
            Hook {
                events: events.clone(),
                mode: Mode {
                    fail_start: false,
                    fail_stop: false,
                },
                name: "first",
            },
        )
        .with_application_hook(Hook {
            events,
            mode,
            name: "last",
        })
}
fn managed_fixture(
    events: Events,
    fail_start: bool,
    fail_stop: bool,
) -> mads_testing::SubjectFixture<Managed> {
    test_fixture()
        .provide(events)
        .provide(Mode {
            fail_start,
            fail_stop,
        })
        .subject::<Managed>()
}
#[tokio::test]
async fn start_and_stop_run_once() {
    let events = Events::default();
    let body_events = events.clone();
    managed_fixture(events.clone(), false, false)
        .run(|app| async move {
            app.resolve::<Managed>().unwrap();
            body_events.0.lock().unwrap().push("body");
        })
        .await
        .unwrap();
    assert_eq!(
        *events.0.lock().unwrap(),
        [
            "start:first",
            "start:last",
            "body",
            "stop:last",
            "stop:first"
        ]
    );
}
#[tokio::test]
async fn panic_still_stops_and_preserves_original_panic() {
    let events = Events::default();
    let result = AssertUnwindSafe(
        managed_fixture(events.clone(), false, false)
            .run(|_| async { panic!("original body panic") }),
    )
    .catch_unwind()
    .await;
    assert_eq!(
        result.unwrap_err().downcast_ref::<&str>(),
        Some(&"original body panic")
    );
    assert_eq!(
        *events.0.lock().unwrap(),
        ["start:first", "start:last", "stop:last", "stop:first"]
    );
}
#[tokio::test]
async fn synchronous_body_factory_panic_still_stops() {
    let events = Events::default();
    let result = AssertUnwindSafe(managed_fixture(events.clone(), false, false).run(|_| {
        panic!("factory panic");
        #[allow(unreachable_code)]
        async {}
    }))
    .catch_unwind()
    .await;
    assert!(result.is_err());
    assert_eq!(
        *events.0.lock().unwrap(),
        ["start:first", "start:last", "stop:last", "stop:first"]
    );
}
#[tokio::test]
async fn startup_failure_rolls_back_without_body() {
    let events = Events::default();
    let result = managed_fixture(events.clone(), true, false)
        .run(|_| async { panic!("body ran") })
        .await;
    assert!(matches!(result, Err(TestError::Mads(_))));
    assert_eq!(
        *events.0.lock().unwrap(),
        ["start:first", "start:last", "stop:first"]
    );
}
#[tokio::test]
async fn shutdown_failure_returns_mads_source() {
    use std::error::Error as _;
    let error = managed_fixture(Events::default(), false, true)
        .run(|_| async {})
        .await
        .unwrap_err();
    assert!(matches!(error, TestError::Mads(_)));
    assert!(
        error
            .source()
            .unwrap()
            .source()
            .unwrap()
            .to_string()
            .contains("stop failed")
    );
}
#[test]
fn panic_and_stop_failure_preserves_panic() {
    if std::env::var_os("MADS_TEST_PANIC_CHILD").is_some() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            managed_fixture(Events::default(), false, true)
                .run(|_| async { panic!("original body panic") })
                .await
                .unwrap();
        });
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "panic_and_stop_failure_preserves_panic",
            "--nocapture",
        ])
        .env("MADS_TEST_PANIC_CHILD", "1")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("original body panic"), "{stderr}");
    assert!(stderr.contains("stop failed"), "{stderr}");
}
