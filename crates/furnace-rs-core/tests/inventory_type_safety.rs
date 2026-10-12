//! Malformed static constructor metadata must fail before application startup.

use furnace_rs_core::{
    ConstructionContext, ErasedProvider, FURNACE004, Furnace, ProviderDescriptor, ProviderFuture,
    ProviderKind, ProviderVisibility, SourceLocation,
};
use std::{any::TypeId, sync::Arc};

struct Expected;

fn mismatched_constructor<'a>(_: &'a ConstructionContext<'a>) -> ProviderFuture<'a> {
    Box::pin(async { Ok(Arc::new("wrong output") as ErasedProvider) })
}

inventory::submit! {
    ProviderDescriptor::new(
        ProviderKind::Provider, "Expected", TypeId::of::<Expected>, &[],
        ProviderVisibility::Private, SourceLocation::new(file!(), line!(), column!()),
        mismatched_constructor,
    )
}

#[tokio::test]
async fn inventory_constructor_type_mismatch_prevents_a_successful_build() {
    let error = Furnace::builder()
        .build()
        .await
        .err()
        .expect("an invalid inventory output must not produce a running application");
    assert_eq!(error.code(), FURNACE004);
}
