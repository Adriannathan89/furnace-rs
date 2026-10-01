//! Runtime integration tests for facade-exported managed-provider attributes.

use std::sync::Arc;

use furnace_rs::common::{HttpMethod, Json, Path, RouteCatalog};
use furnace_rs::core::{
    Catalog as CoreCatalog, Furnace as CoreMads, ProviderKind, ProviderOrigin, ProviderVisibility,
};

fn framework_result() -> furnace_rs::core::Result<()> {
    Ok(())
}

mod declarations {
    use furnace_rs::prelude::*;

    #[cauldron]
    pub(super) struct PreludeCauldron;

    impl furnace_rs::core::Cauldron for PreludeCauldron {
        fn register(self) -> furnace_rs::core::CauldronRegistration<Self> {
            furnace_rs::core::CauldronRegistration::new(self)
        }
    }
}

#[test]
fn prelude_exposes_the_http_runtime_surface() {
    use furnace_rs::prelude::{
        BadRequest, Cauldron, CauldronGraph, CauldronImportDescriptor, CauldronImportEdge,
        CauldronNode, Conflict, Created, Forbidden, Furnace, FurnaceBurnExt, Header, HttpError,
        HttpResult, HttpRuntimeError, Input, InternalError, Json, NoContent, NotFound, Path,
        ProviderOwnership, Query, Request, SourcedValidationIssue, Unauthorized, ValidatedJson,
        ValidatedPath, ValidatedQuery, ValidationError, ValidationErrors, ValidationIssue,
        ValidationPathSegment, ValidationResult, ValidationSource, build_router, configure_router,
        serve, serve_router,
    };

    fn assert_module<T: Cauldron>() {}
    fn assert_input<T: Input>() {}

    let _ = std::any::TypeId::of::<BadRequest>();
    let _ = std::any::TypeId::of::<Unauthorized>();
    let _ = std::any::TypeId::of::<Forbidden>();
    let _ = std::any::TypeId::of::<NotFound>();
    let _ = std::any::TypeId::of::<Conflict>();
    let _ = std::any::TypeId::of::<ValidationError>();
    let _ = std::any::TypeId::of::<InternalError>();
    let _ = std::any::TypeId::of::<Created<NoContent>>();
    let _ = std::any::TypeId::of::<Header<furnace_rs::common::headers::ContentType>>();
    let _ = std::any::TypeId::of::<HttpError>();
    let _ = std::any::TypeId::of::<HttpResult<NoContent>>();
    let _ = std::any::TypeId::of::<Json<NoContent>>();
    let _ = std::any::TypeId::of::<Path<String>>();
    let _ = std::any::TypeId::of::<Query<String>>();
    let _ = std::any::TypeId::of::<Request>();
    let _ = std::any::TypeId::of::<ValidatedJson<String>>();
    let _ = std::any::TypeId::of::<ValidatedQuery<String>>();
    let _ = std::any::TypeId::of::<ValidatedPath<String>>();
    let _ = std::any::TypeId::of::<SourcedValidationIssue>();
    let _ = std::any::TypeId::of::<ValidationErrors>();
    let _ = std::any::TypeId::of::<ValidationIssue>();
    let _ = std::any::TypeId::of::<ValidationPathSegment>();
    let _ = std::any::TypeId::of::<ValidationResult>();
    let _ = std::any::TypeId::of::<ValidationSource>();
    assert_input::<String>();
    assert_module::<declarations::PreludeCauldron>();
    let _ = std::any::TypeId::of::<CauldronGraph>();
    let _ = std::any::TypeId::of::<CauldronImportDescriptor>();
    let _ = std::any::TypeId::of::<CauldronImportEdge>();
    let _ = std::any::TypeId::of::<CauldronNode>();
    let _ = std::any::TypeId::of::<ProviderOwnership>();
    let _ = std::any::TypeId::of::<HttpRuntimeError>();
    let _ = build_router;
    let _ = configure_router;
    let _ = |application: furnace_rs::core::Furnace| serve(application, "127.0.0.1:0");
    let _ = |application: furnace_rs::core::Furnace, router: furnace_rs::axum::Router| {
        serve_router(application, router, "127.0.0.1:0")
    };
    let _ = <Furnace as FurnaceBurnExt>::burn::<declarations::PreludeCauldron>;

    fn assert_root_cauldron<T: furnace_rs::Cauldron>() {}
    assert_root_cauldron::<declarations::PreludeCauldron>();
    let _ = std::any::TypeId::of::<furnace_rs::CauldronGraph>();
    let _ = std::any::TypeId::of::<furnace_rs::CauldronImportDescriptor>();
    let _ = std::any::TypeId::of::<furnace_rs::CauldronImportEdge>();
    let _ = std::any::TypeId::of::<furnace_rs::CauldronNode>();
    let _ = std::any::TypeId::of::<furnace_rs::ProviderOwnership>();
    let _ = std::any::TypeId::of::<furnace_rs::BadRequest>();
    let _ = std::any::TypeId::of::<furnace_rs::Unauthorized>();
    let _ = std::any::TypeId::of::<furnace_rs::Forbidden>();
    let _ = std::any::TypeId::of::<furnace_rs::NotFound>();
    let _ = std::any::TypeId::of::<furnace_rs::Conflict>();
    let _ = std::any::TypeId::of::<furnace_rs::ValidationError>();
    let _ = std::any::TypeId::of::<furnace_rs::InternalError>();
    let _ = std::any::TypeId::of::<furnace_rs::SourcedValidationIssue>();
    let _ = std::any::TypeId::of::<furnace_rs::ValidationSource>();
    let _ = std::any::TypeId::of::<furnace_rs::ValidatedJson<String>>();
    let _ = std::any::TypeId::of::<furnace_rs::ValidatedQuery<String>>();
    let _ = std::any::TypeId::of::<furnace_rs::ValidatedPath<String>>();
    let _ = furnace_rs::build_router;
    let _ = furnace_rs::configure_router;
    let _ = |application: furnace_rs::core::Furnace| furnace_rs::serve(application, "127.0.0.1:0");
    let _ = |application: furnace_rs::core::Furnace, router: furnace_rs::axum::Router| {
        furnace_rs::serve_router(application, router, "127.0.0.1:0")
    };
    let _ = <furnace_rs::core::Furnace as furnace_rs::FurnaceBurnExt>::burn::<
        declarations::PreludeCauldron,
    >;
    let _ = framework_result;
    let _: furnace_rs::common::axum::Router = furnace_rs::common::axum::Router::new();
}

#[cfg(feature = "cookies")]
#[tokio::test]
async fn prelude_exposes_cookie_types_through_native_axum() {
    use furnace_rs::axum::{
        Router,
        body::Body,
        http::{Request, header::SET_COOKIE},
        routing::get,
    };
    use furnace_rs::prelude::*;
    use tower::ServiceExt;

    async fn handler(jar: CookieJar) -> (CookieJar, &'static str) {
        let session = Cookie::build(("session", "opaque-token"))
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Lax)
            .max_age(cookie::time::Duration::minutes(5))
            .build();
        (jar.add(session), "ok")
    }

    let response = Router::new()
        .route("/session", get(handler))
        .oneshot(
            Request::builder()
                .uri("/session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.headers().contains_key(SET_COOKIE));
    let _ = std::any::TypeId::of::<Expiration>();
    let _ = std::any::TypeId::of::<CookieError>();
    let _ = std::any::TypeId::of::<CookieErrorKind>();
    let _ = std::any::TypeId::of::<CookieRejection>();
    let _: CookieResult<()> = Ok(());
    assert_eq!(FURNACE110.as_str(), "FURNACE110");
    let _ = std::any::TypeId::of::<furnace_rs::Cookie<'static>>();
    let _ = std::any::TypeId::of::<furnace_rs::CookieJar>();
    let _: furnace_rs::cookie::time::Duration = cookie::time::Duration::seconds(1);
}

#[test]
fn prelude_exposes_core_types_and_bare_attributes() {
    use furnace_rs::prelude::*;

    mod core_declarations {
        use furnace_rs::prelude::*;

        #[cauldron]
        struct PreludeCauldron;

        impl furnace_rs::core::Cauldron for PreludeCauldron {
            fn register(self) -> furnace_rs::core::CauldronRegistration<Self> {
                self.provide::<usize>()
                    .provide::<PreludeRepository>()
                    .provide::<PreludeService>()
                    .controller::<PreludeController>()
            }
        }

        #[element]
        fn prelude_value() -> usize {
            1
        }

        #[storage]
        struct PreludeRepository;

        #[burner]
        struct PreludeService;

        #[allow(dead_code)]
        #[routes(prefix = "/prelude")]
        trait PreludeRoutes {
            #[get("/")]
            async fn index(&self);
        }

        #[controller(routes = [PreludeRoutes])]
        struct PreludeController;

        impl PreludeRoutes for PreludeController {
            async fn index(&self) {}
        }

        #[cfg(feature = "runtime-tokio")]
        #[main]
        async fn main() {}
    }

    let _ = std::any::TypeId::of::<Furnace>();
    let _ = std::any::TypeId::of::<Config>();
    let _ = std::any::TypeId::of::<Diagnostic>();
    let _ = std::any::TypeId::of::<Catalog>();
    let _ = std::any::TypeId::of::<LifecycleState>();
    let _: furnace_rs::core::FurnaceBuilder = Furnace::builder();
    let _: furnace_rs::core::FurnaceBuilder = Furnace::builder_with_config(Config::empty());
}

#[furnace_rs::cauldron]
struct FacadeCauldron;

impl furnace_rs::core::Cauldron for FacadeCauldron {
    fn register(self) -> furnace_rs::core::CauldronRegistration<Self> {
        self.provide::<PublicGraphService>()
            .provide::<u16>()
            .provide::<FacadeRepository>()
            .provide::<QueryUsecase>()
            .provide::<CommandUsecase>()
            .controller::<FacadeController>()
            .provide::<GroupedFallibleProvider>()
            .provide::<FacadeService>()
            .controller::<RootController>()
            .export::<PublicGraphService>()
    }
}

/// Public managed service used to verify facade visibility metadata.
#[furnace_rs::burner]
pub struct PublicGraphService;

#[furnace_rs::element]
pub(crate) fn restricted_graph_value() -> u16 {
    16
}

#[furnace_rs::storage]
struct FacadeRepository;

#[derive(Clone)]
struct Clock;

struct GroupedFallibleProvider;

#[furnace_rs::burner]
struct QueryUsecase;

#[furnace_rs::burner]
struct CommandUsecase;

#[furnace_rs::routes(prefix = "/users")]
trait QueryRoutes {
    #[furnace_rs::get("/:id")]
    async fn get_user(&self, id: Path<i64>) -> String;
}

#[furnace_rs::routes]
trait CommandRoutes {
    #[furnace_rs::post("/users")]
    async fn create_user(&self, id: Json<i64>) -> String;
}

#[furnace_rs::controller(routes = [QueryRoutes, CommandRoutes])]
struct FacadeController {
    query: QueryUsecase,
    command: CommandUsecase,
}

impl QueryRoutes for FacadeController {
    async fn get_user(&self, Path(id): Path<i64>) -> String {
        let _query = &self.query;
        id.to_string()
    }
}

impl CommandRoutes for FacadeController {
    async fn create_user(&self, Json(id): Json<i64>) -> String {
        let _command = &self.command;
        id.to_string()
    }
}

#[allow(clippy::result_large_err, unused_parens)]
#[furnace_rs::element]
fn grouped_fallible_provider() -> (furnace_rs::core::Result<GroupedFallibleProvider>) {
    Ok(GroupedFallibleProvider)
}

#[furnace_rs::burner]
struct FacadeService {
    repository: FacadeRepository,
    clock: Clock,
}

impl FacadeService {
    fn inner_address(&self) -> *const () {
        std::ptr::from_ref(&**self).cast()
    }

    fn has_dependencies(&self) -> bool {
        let _repository = &self.repository;
        let _clock = &self.clock;
        true
    }
}

#[test]
fn facade_attributes_register_stable_descriptors() {
    let cauldron_names: Vec<_> = CoreCatalog::cauldrons()
        .into_iter()
        .map(|descriptor| descriptor.type_name())
        .collect();
    let providers = CoreCatalog::providers();

    assert!(cauldron_names.contains(&"facade::FacadeCauldron"));
    assert!(providers.iter().any(|descriptor| {
        descriptor.type_name() == "facade::FacadeRepository"
            && descriptor.kind() == ProviderKind::Repository
    }));
    assert!(providers.iter().any(|descriptor| {
        descriptor.type_name() == "facade::FacadeService"
            && descriptor.kind() == ProviderKind::Service
    }));
    assert!(providers.iter().any(|descriptor| {
        descriptor.type_name() == "facade::FacadeController"
            && descriptor.kind() == ProviderKind::Service
    }));
    assert_eq!(
        CoreCatalog::provider_for::<PublicGraphService>()
            .expect("public service descriptor should exist")
            .visibility(),
        ProviderVisibility::Public,
    );
    assert_eq!(
        CoreCatalog::provider_for::<u16>()
            .expect("restricted provider descriptor should exist")
            .visibility(),
        ProviderVisibility::Private,
    );
}

#[test]
fn controller_dependencies_follow_source_field_order() {
    let descriptor = CoreCatalog::provider_for::<FacadeController>()
        .expect("the facade controller descriptor should be registered");
    let dependency_names: Vec<_> = descriptor
        .dependencies()
        .iter()
        .map(|dependency| dependency.type_name())
        .collect();

    assert_eq!(dependency_names, ["QueryUsecase", "CommandUsecase"]);
}

#[test]
fn controller_keeps_deterministic_route_metadata() {
    let routes = RouteCatalog::routes_for::<FacadeController>();

    assert_eq!(routes.len(), 2);
    assert_eq!(routes[0].method(), HttpMethod::Get);
    assert_eq!(routes[0].prefix(), "/users");
    assert_eq!(routes[0].path(), "/:id");
    assert_eq!(routes[0].full_path(), "/users/:id");
    assert_eq!(routes[0].handler(), "get_user");
    assert_eq!(routes[1].method(), HttpMethod::Post);
    assert_eq!(routes[1].full_path(), "/users");
    assert!(RouteCatalog::validate_controller::<FacadeController>().is_ok());
}

#[allow(dead_code)]
#[furnace_rs::routes(prefix = "/")]
trait RootRoutes {
    #[furnace_rs::get("/health")]
    async fn health(&self);
}

#[furnace_rs::controller(routes = [RootRoutes])]
struct RootController;

impl RootRoutes for RootController {
    async fn health(&self) {}
}

#[test]
fn root_prefix_does_not_create_an_ambiguous_double_slash_path() {
    let routes = RouteCatalog::routes_for::<RootController>();
    assert_eq!(routes[0].full_path(), "/health");
}

#[test]
fn service_dependencies_follow_source_field_order() {
    let descriptor = CoreCatalog::provider_for::<FacadeService>()
        .expect("the facade service descriptor should be registered");
    let dependency_names: Vec<_> = descriptor
        .dependencies()
        .iter()
        .map(|dependency| dependency.type_name())
        .collect();

    assert_eq!(dependency_names, ["FacadeRepository", "Clock"]);
}

#[tokio::test]
async fn cloned_service_handles_share_the_inner_allocation() {
    let mut builder = CoreMads::builder();
    builder
        .provide(Clock)
        .expect("clock insertion should succeed");

    let application = builder
        .build()
        .await
        .expect("the application graph should build");
    let service = application
        .context()
        .resolve::<FacadeService>()
        .expect("the constructed service should resolve");
    let graph_service = application
        .graph()
        .provider::<FacadeService>()
        .expect("the facade service should be in the graph");
    assert_eq!(graph_service.origin(), ProviderOrigin::Service);
    assert_eq!(graph_service.visibility(), ProviderVisibility::Private);
    assert!(application.graph().dependencies().iter().any(|edge| {
        edge.provider_type_name().ends_with("FacadeService")
            && edge.dependency_type_name().ends_with("FacadeRepository")
    }));
    let cloned = service.as_ref().clone();

    assert!(service.has_dependencies());
    assert_eq!(service.inner_address(), cloned.inner_address());
    assert!(Arc::ptr_eq(
        &service,
        &application.context().resolve().unwrap()
    ));
}

#[tokio::test]
async fn controller_constructs_after_multiple_usecases() {
    let mut builder = CoreMads::builder();
    builder
        .provide(Clock)
        .expect("clock insertion should succeed");
    builder
        .construct::<QueryUsecase>()
        .await
        .expect("query use case construction should succeed");
    builder
        .construct::<CommandUsecase>()
        .await
        .expect("command use case construction should succeed");
    builder
        .construct::<FacadeController>()
        .await
        .expect("controller construction should succeed");

    let application = builder
        .build()
        .await
        .expect("the application graph should build");
    let controller = application
        .context()
        .resolve::<FacadeController>()
        .expect("the constructed controller should resolve");
    let cloned = controller.as_ref().clone();

    assert_eq!(controller.get_user(Path(7)).await, "7");
    assert_eq!(controller.create_user(Json(8)).await, "8");
    assert!(std::ptr::eq(&**controller, &*cloned));
}

#[tokio::test]
async fn grouped_furnace_results_register_their_success_type() {
    let mut builder = CoreMads::builder();
    builder
        .provide(Clock)
        .expect("clock insertion should succeed");

    builder
        .construct::<GroupedFallibleProvider>()
        .await
        .expect("a grouped FURNACE Result provider should construct its success type");

    let application = builder
        .build()
        .await
        .expect("the application graph should build");
    assert!(
        application
            .context()
            .resolve::<GroupedFallibleProvider>()
            .is_ok()
    );
}
