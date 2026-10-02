//! Confirms synchronous and asynchronous provider functions compile.

use furnace_rs::core::{Config, Result};

struct AsyncDirect;

struct SyncFallible;

struct CoreFallible;

fn sync_value() -> String {
    "value".to_owned()
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct SyncValueInjector;
impl furnace_rs::core::Injector<String> for SyncValueInjector {
    type Dependencies = ();
    async fn inject((): Self::Dependencies) -> furnace_rs::core::Result<String> {
        Ok(sync_value())
    }
    fn descriptor() -> &'static furnace_rs::core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_SYNC_VALUE
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_SYNC_VALUE: furnace_rs::core::ProviderDescriptor =
    furnace_rs::core::__private::InjectorMetadata::<String, SyncValueInjector>::DESCRIPTOR
        .with_authored_type_name("String")
        .with_namespace(module_path!())
        .with_visibility(furnace_rs::core::ProviderVisibility::Private)
        .with_location(furnace_rs::core::SourceLocation::new(
            file!(),
            line!(),
            column!(),
        ));
furnace_rs::core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_SYNC_VALUE }

async fn async_value(config: Config) -> furnace_rs::core::Result<usize> {
    Ok(config.len())
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct AsyncValueInjector;
impl furnace_rs::core::Injector<usize> for AsyncValueInjector {
    type Dependencies = (Config,);
    async fn inject((dependency_0,): Self::Dependencies) -> furnace_rs::core::Result<usize> {
        async_value(dependency_0).await
    }
    fn descriptor() -> &'static furnace_rs::core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_ASYNC_VALUE
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_ASYNC_VALUE: furnace_rs::core::ProviderDescriptor =
    furnace_rs::core::__private::InjectorMetadata::<usize, AsyncValueInjector>::DESCRIPTOR
        .with_authored_type_name("usize")
        .with_namespace(module_path!())
        .with_visibility(furnace_rs::core::ProviderVisibility::Private)
        .with_location(furnace_rs::core::SourceLocation::new(
            file!(),
            line!(),
            column!(),
        ));
furnace_rs::core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_ASYNC_VALUE }

async fn async_direct() -> AsyncDirect {
    AsyncDirect
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct AsyncDirectInjector;
impl furnace_rs::core::Injector<AsyncDirect> for AsyncDirectInjector {
    type Dependencies = ();
    async fn inject((): Self::Dependencies) -> furnace_rs::core::Result<AsyncDirect> {
        Ok(async_direct().await)
    }
    fn descriptor() -> &'static furnace_rs::core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_ASYNC_DIRECT
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_ASYNC_DIRECT: furnace_rs::core::ProviderDescriptor =
    furnace_rs::core::__private::InjectorMetadata::<AsyncDirect, AsyncDirectInjector>::DESCRIPTOR
        .with_authored_type_name("AsyncDirect")
        .with_namespace(module_path!())
        .with_visibility(furnace_rs::core::ProviderVisibility::Private)
        .with_location(furnace_rs::core::SourceLocation::new(
            file!(),
            line!(),
            column!(),
        ));
furnace_rs::core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_ASYNC_DIRECT }

fn sync_fallible() -> Result<SyncFallible> {
    Ok(SyncFallible)
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct SyncFallibleInjector;
impl furnace_rs::core::Injector<SyncFallible> for SyncFallibleInjector {
    type Dependencies = ();
    async fn inject((): Self::Dependencies) -> furnace_rs::core::Result<SyncFallible> {
        sync_fallible()
    }
    fn descriptor() -> &'static furnace_rs::core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_SYNC_FALLIBLE
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_SYNC_FALLIBLE: furnace_rs::core::ProviderDescriptor =
    furnace_rs::core::__private::InjectorMetadata::<SyncFallible, SyncFallibleInjector>::DESCRIPTOR
        .with_authored_type_name("SyncFallible")
        .with_namespace(module_path!())
        .with_visibility(furnace_rs::core::ProviderVisibility::Private)
        .with_location(furnace_rs::core::SourceLocation::new(
            file!(),
            line!(),
            column!(),
        ));
furnace_rs::core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_SYNC_FALLIBLE }

fn core_fallible() -> furnace_rs_core::Result<CoreFallible> {
    Ok(CoreFallible)
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct CoreFallibleInjector;
impl furnace_rs::core::Injector<CoreFallible> for CoreFallibleInjector {
    type Dependencies = ();
    async fn inject((): Self::Dependencies) -> furnace_rs::core::Result<CoreFallible> {
        core_fallible()
    }
    fn descriptor() -> &'static furnace_rs::core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_CORE_FALLIBLE
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_CORE_FALLIBLE: furnace_rs::core::ProviderDescriptor =
    furnace_rs::core::__private::InjectorMetadata::<CoreFallible, CoreFallibleInjector>::DESCRIPTOR
        .with_authored_type_name("CoreFallible")
        .with_namespace(module_path!())
        .with_visibility(furnace_rs::core::ProviderVisibility::Private)
        .with_location(furnace_rs::core::SourceLocation::new(
            file!(),
            line!(),
            column!(),
        ));
furnace_rs::core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_CORE_FALLIBLE }

fn __furnace_construct_sync_value() {}

fn main() {
    let _user_function: fn() = __furnace_construct_sync_value;
}
