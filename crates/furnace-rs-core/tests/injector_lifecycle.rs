//! Injector resources retain native outputs, ordered hooks, and error cleanup.
use furnace_rs_core::{
    ApplicationContext, Cauldron, CauldronRegistration, Diagnostic, Error, FURNACE020, Furnace,
    Injector, LifecycleFuture, LifecycleHook, LifecycleResource, Result,
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct Events(Arc<Mutex<Vec<&'static str>>>);
impl Events {
    fn push(&self, event: &'static str) {
        self.0.lock().unwrap().push(event);
    }
    fn snapshot(&self) -> Vec<&'static str> {
        self.0.lock().unwrap().clone()
    }
}
impl Injector for Events {
    type Dependencies = ();
    async fn inject((): ()) -> Result<Self> {
        Ok(Self::default())
    }
}
struct DropToken(Events);
impl Drop for DropToken {
    fn drop(&mut self) {
        self.0.push("drop");
    }
}
#[derive(Clone)]
struct Resource {
    events: Events,
    token: Arc<DropToken>,
}
struct Hook {
    events: Events,
    name: &'static str,
    fail: bool,
}
impl LifecycleHook for Hook {
    fn name(&self) -> &str {
        self.name
    }
    fn start<'a>(&'a self, _: &'a ApplicationContext) -> LifecycleFuture<'a> {
        Box::pin(async move {
            self.events.push(if self.name == "resource" {
                "start:resource"
            } else {
                "start:application"
            });
            if self.fail { Err(error()) } else { Ok(()) }
        })
    }
    fn stop<'a>(&'a self, _: &'a ApplicationContext) -> LifecycleFuture<'a> {
        Box::pin(async move {
            self.events.push(if self.name == "resource" {
                "stop:resource"
            } else {
                "stop:application"
            });
            Ok(())
        })
    }
}
impl Injector for Resource {
    type Dependencies = (Events,);
    async fn inject((events,): Self::Dependencies) -> Result<Self> {
        events.push("inject");
        Ok(Self {
            token: Arc::new(DropToken(events.clone())),
            events,
        })
    }
    fn lifecycle(value: Self) -> LifecycleResource<Self> {
        value.events.push("attach");
        let hook = Hook {
            events: value.events.clone(),
            name: "resource",
            fail: false,
        };
        LifecycleResource::new(value).with_infrastructure_hook("injector.resource", hook)
    }
}
struct Failing;
impl Injector for Failing {
    type Dependencies = (Resource,);
    async fn inject((resource,): Self::Dependencies) -> Result<Self> {
        assert!(Arc::strong_count(&resource.token) >= 1);
        resource.events.push("fail");
        Err(error())
    }
}
fn error() -> Error {
    Error::new(Diagnostic::new(
        FURNACE020,
        "expected failure",
        "test failure",
    ))
}

#[furnace_rs_core::cauldron]
struct Root;
impl Cauldron for Root {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide_with::<Events, Events>()
            .provide_with::<Resource, Resource>()
    }
}
#[furnace_rs_core::cauldron]
struct FailureRoot;
impl Cauldron for FailureRoot {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide_with::<Events, Events>()
            .provide_with::<Resource, Resource>()
            .provide_with::<Failing, Failing>()
    }
}
fn builder<M: Cauldron>(events: &Events) -> furnace_rs_core::FurnaceBuilder {
    let mut builder = Furnace::builder();
    builder.root::<M>().unwrap();
    builder.provide(events.clone()).unwrap();
    builder
}

#[tokio::test]
async fn hooks_attach_once_and_wrap_application_lifecycle() {
    let events = Events::default();
    let mut builder = builder::<Root>(&events);
    builder.lifecycle_hook(Hook {
        events: events.clone(),
        name: "application",
        fail: false,
    });
    assert!(builder.analyze().is_valid());
    assert!(events.snapshot().is_empty());
    let mut app = builder.build().await.unwrap();
    assert!(app.context().resolve::<Resource>().is_ok());
    assert!(
        app.context()
            .resolve::<LifecycleResource<Resource>>()
            .is_err()
    );
    app.start().await.unwrap();
    app.shutdown().await.unwrap();
    drop(app);
    assert_eq!(
        events.snapshot(),
        [
            "inject",
            "attach",
            "start:resource",
            "start:application",
            "stop:application",
            "stop:resource",
            "drop"
        ]
    );
}
#[tokio::test]
async fn later_construction_failure_drops_resource_without_starting_hooks() {
    let events = Events::default();
    assert!(builder::<FailureRoot>(&events).build().await.is_err());
    assert_eq!(events.snapshot(), ["inject", "attach", "fail", "drop"]);
}
#[tokio::test]
async fn failed_application_start_rolls_back_the_resource_hook() {
    let events = Events::default();
    let mut builder = builder::<Root>(&events);
    builder.lifecycle_hook(Hook {
        events: events.clone(),
        name: "application",
        fail: true,
    });
    let mut app = builder.build().await.unwrap();
    assert!(app.start().await.is_err());
    drop(app);
    assert_eq!(
        events.snapshot(),
        [
            "inject",
            "attach",
            "start:resource",
            "start:application",
            "stop:resource",
            "drop"
        ]
    );
}
#[tokio::test]
async fn supplied_resource_has_no_injector_hooks() {
    let events = Events::default();
    let mut builder = builder::<Root>(&events);
    builder
        .provide(Resource {
            token: Arc::new(DropToken(events.clone())),
            events: events.clone(),
        })
        .unwrap();
    let mut app = builder.build().await.unwrap();
    app.start().await.unwrap();
    app.shutdown().await.unwrap();
    assert!(events.snapshot().is_empty());
    drop(app);
    assert_eq!(events.snapshot(), ["drop"]);
}
#[test]
fn low_level_native_catalog_bridge_is_available_without_constructing() {
    let descriptor =
        &furnace_rs_core::__private::InjectorMetadata::<Resource, Resource>::DESCRIPTOR;
    assert_eq!(descriptor.type_id(), std::any::TypeId::of::<Resource>());
    assert!(descriptor.lifecycle_constructor().is_some());
}
