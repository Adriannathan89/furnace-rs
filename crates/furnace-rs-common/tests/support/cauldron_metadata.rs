//! Raw metadata fixtures for runtime diagnostics that declarative syntax rejects.
// Keep malformed graphs out of the supported application registration API.
macro_rules! cauldron_metadata {
    ($visibility:vis struct $name:ident; $definition:expr) => {
        $visibility struct $name;
        impl furnace_rs_core::__private::ModuleMetadata for $name {
            const KEY: &'static str = concat!(module_path!(), "::", stringify!($name));
        }
        impl furnace_rs_core::__private::ModuleContract for $name {
            type Metadata = Self;
        }
        furnace_rs_core::__private::inventory::submit! {
            furnace_rs_core::CauldronDescriptor::new(
                concat!(module_path!(), "::", stringify!($name)),
                || std::any::TypeId::of::<$name>(),
                furnace_rs_core::SourceLocation::new(file!(), line!(), column!()),
            ).with_namespace(module_path!()).with_registration(|| $definition.into_definition())
        }
    };
}
