//! Integration test for complete-catalog construction.

use std::sync::atomic::{AtomicUsize, Ordering};

use furnace_rs::core::Furnace;

static CONSTRUCTIONS: AtomicUsize = AtomicUsize::new(0);

struct OtherwiseUnused;

fn otherwise_unused() -> OtherwiseUnused {
    CONSTRUCTIONS.fetch_add(1, Ordering::SeqCst);
    OtherwiseUnused
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct OtherwiseUnusedInjector;
impl furnace_rs::core::Injector<OtherwiseUnused> for OtherwiseUnusedInjector {
    type Dependencies = ();
    async fn inject((): Self::Dependencies) -> furnace_rs::core::Result<OtherwiseUnused> {
        Ok(otherwise_unused())
    }
    fn descriptor() -> &'static furnace_rs::core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_OTHERWISE_UNUSED
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_OTHERWISE_UNUSED : furnace_rs :: core :: ProviderDescriptor = furnace_rs :: core :: __private :: InjectorMetadata :: < OtherwiseUnused , OtherwiseUnusedInjector > :: DESCRIPTOR . with_authored_type_name (stringify ! (OtherwiseUnused)) . with_namespace (module_path ! ()) . with_visibility (furnace_rs :: core :: ProviderVisibility :: Private) . with_location (furnace_rs :: core :: SourceLocation :: new (file ! () , line ! () , column ! ())) ;
furnace_rs::core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_OTHERWISE_UNUSED }

#[tokio::test]
async fn automatic_build_constructs_an_unreferenced_catalog_provider() {
    CONSTRUCTIONS.store(0, Ordering::SeqCst);
    let application = Furnace::builder()
        .build()
        .await
        .expect("the complete catalog should build");

    assert_eq!(CONSTRUCTIONS.load(Ordering::SeqCst), 1);
    assert!(application.context().resolve::<OtherwiseUnused>().is_ok());
}
