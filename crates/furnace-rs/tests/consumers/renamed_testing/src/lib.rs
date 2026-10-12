//! A consumer with renamed dependencies and no direct Tokio dependency.
#[framework::burner]
struct Service;
#[framework::test]
async fn renamed_fixture_runs() -> fixtures::TestResult<()> {
    test_fixture().subject::<Service>().run(|context| async move {
        assert!(context.resolve::<Service>().is_ok());
    }).await
}
