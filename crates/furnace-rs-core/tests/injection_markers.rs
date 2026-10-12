//! Explicit markers preserve typed tuple injection and shared instances.
use furnace_rs_core::Injector;
use std::sync::Arc;
#[furnace_rs_core::burner]
struct Marked {
    #[inject]
    value: Arc<String>,
    #[inject]
    second: Arc<String>,
}
#[furnace_rs_core::storage]
struct Repository {
    #[inject]
    value: Arc<String>,
}
#[tokio::test]
async fn marked_fields_share_the_injected_instances() {
    let value = Arc::new(String::from("same application value"));
    let marked = Marked::inject((value.clone(), value.clone()))
        .await
        .unwrap();
    let repository = Repository::inject((value.clone(),)).await.unwrap();
    assert!(Arc::ptr_eq(&marked.value, &value));
    assert!(Arc::ptr_eq(&marked.second, &value));
    assert!(Arc::ptr_eq(&repository.value, &value));
    assert!(Arc::ptr_eq(&marked.clone().value, &value));
}
