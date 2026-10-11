//! Explicit markers preserve legacy tuple injection and shared instances.
use furnace_rs_core::Injector;
use std::sync::Arc;
#[furnace_rs_core::burner]
struct Marked {
    #[inject]
    value: Arc<String>,
    legacy: Arc<String>,
}
#[furnace_rs_core::storage]
struct Legacy {
    value: Arc<String>,
}
#[tokio::test]
async fn marked_and_unmarked_fields_share_the_injected_instances() {
    let value = Arc::new(String::from("same application value"));
    let marked = Marked::inject((value.clone(), value.clone()))
        .await
        .unwrap();
    let legacy = Legacy::inject((value.clone(),)).await.unwrap();
    assert!(Arc::ptr_eq(&marked.value, &value));
    assert!(Arc::ptr_eq(&marked.legacy, &value));
    assert!(Arc::ptr_eq(&legacy.value, &value));
    assert!(Arc::ptr_eq(&marked.clone().value, &value));
}
