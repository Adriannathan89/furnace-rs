//! Integration test proving validation precedes construction.

use std::any::TypeId;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use furnace_rs_core::{
    ConstructionContext, DependencyDescriptor, ErasedProvider, FURNACE003, Furnace,
    ProviderDescriptor, ProviderFuture, ProviderKind, ProviderVisibility, SourceLocation,
};

static CONSTRUCTIONS: AtomicUsize = AtomicUsize::new(0);

struct MissingDependency;
struct InvalidService;

fn missing_type_id() -> TypeId {
    TypeId::of::<MissingDependency>()
}

fn service_type_id() -> TypeId {
    TypeId::of::<InvalidService>()
}

fn invalid_constructor<'a>(_: &'a ConstructionContext<'a>) -> ProviderFuture<'a> {
    Box::pin(async {
        CONSTRUCTIONS.fetch_add(1, Ordering::SeqCst);
        Ok(Arc::new(InvalidService) as ErasedProvider)
    })
}

static DEPENDENCIES: [DependencyDescriptor; 1] = [DependencyDescriptor::new(
    "invalid::MissingDependency",
    missing_type_id,
)];

inventory::submit! {
    ProviderDescriptor::new(
        ProviderKind::Service,
        "invalid::InvalidService",
        service_type_id,
        &DEPENDENCIES,
        ProviderVisibility::Private,
        SourceLocation::new(file!(), line!(), column!()),
        invalid_constructor,
    )
}

#[tokio::test]
async fn invalid_graph_runs_zero_constructors() {
    CONSTRUCTIONS.store(0, Ordering::SeqCst);
    let Err(error) = Furnace::builder().build().await else {
        panic!("the unresolved dependency must reject the graph");
    };

    assert_eq!(error.code(), FURNACE003);
    assert_eq!(CONSTRUCTIONS.load(Ordering::SeqCst), 0);
}
