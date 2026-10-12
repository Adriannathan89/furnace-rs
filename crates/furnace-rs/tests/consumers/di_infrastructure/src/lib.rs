//! Private module/constructor metadata crosses a public re-export.
#![deny(warnings)]
mod private {
    use furnace_rs_core::{cauldron, Injector, Result};
    use std::sync::Arc;
    #[derive(Clone)] struct State;
    struct StateFactory;
    impl Injector<State> for StateFactory {
        type Dependencies = ();
        async fn inject(_: ()) -> Result<State> { Ok(State) }
    }
    struct ConfigFactory;
    impl Injector<furnace_rs_core::Config> for ConfigFactory {
        type Dependencies = (State,);
        async fn inject(_: Self::Dependencies) -> Result<furnace_rs_core::Config> { Ok(furnace_rs_core::Config::empty()) }
    }
    type Settings = furnace_rs_core::Config;
    struct NativeFactory;
    impl Injector<Arc<String>> for NativeFactory {
        type Dependencies = (State,);
        async fn inject(_: Self::Dependencies) -> Result<Arc<String>> { Ok(Arc::new(String::new())) }
    }
    #[cauldron(providers=[State => StateFactory, Arc<String> => NativeFactory, Settings => ConfigFactory], exports=[Arc<String>, Settings], global)]
    struct Database;
    #[cauldron(imports=[Database])]
    pub struct Parent;
}
pub use private::Parent;
