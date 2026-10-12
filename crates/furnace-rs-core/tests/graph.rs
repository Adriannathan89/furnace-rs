//! Integration tests for public graph analysis and inspection.

use furnace_rs_core::{ConstructionStep, Furnace, ProviderOrigin, ProviderState};

#[derive(Clone)]
struct GraphDatabase;

struct GraphRepository {
    _database: GraphDatabase,
}

fn graph_database() -> GraphDatabase {
    GraphDatabase
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct GraphDatabaseInjector;
impl furnace_rs_core::Injector<GraphDatabase> for GraphDatabaseInjector {
    type Dependencies = ();
    async fn inject((): Self::Dependencies) -> furnace_rs_core::Result<GraphDatabase> {
        Ok(graph_database())
    }
    fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_GRAPH_DATABASE
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_GRAPH_DATABASE : furnace_rs_core :: ProviderDescriptor = furnace_rs_core :: __private :: InjectorMetadata :: < GraphDatabase , GraphDatabaseInjector > :: DESCRIPTOR . with_authored_type_name (stringify ! (GraphDatabase)) . with_namespace (module_path ! ()) . with_visibility (furnace_rs_core :: ProviderVisibility :: Private) . with_location (furnace_rs_core :: SourceLocation :: new (file ! () , line ! () , column ! ())) ;
furnace_rs_core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_GRAPH_DATABASE }

fn graph_repository(database: GraphDatabase) -> GraphRepository {
    GraphRepository {
        _database: database,
    }
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct GraphRepositoryInjector;
impl furnace_rs_core::Injector<GraphRepository> for GraphRepositoryInjector {
    type Dependencies = (GraphDatabase,);
    async fn inject(
        (dependency_0,): Self::Dependencies,
    ) -> furnace_rs_core::Result<GraphRepository> {
        Ok(graph_repository(dependency_0))
    }
    fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_GRAPH_REPOSITORY
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_GRAPH_REPOSITORY : furnace_rs_core :: ProviderDescriptor = furnace_rs_core :: __private :: InjectorMetadata :: < GraphRepository , GraphRepositoryInjector > :: DESCRIPTOR . with_authored_type_name (stringify ! (GraphRepository)) . with_namespace (module_path ! ()) . with_visibility (furnace_rs_core :: ProviderVisibility :: Private) . with_location (furnace_rs_core :: SourceLocation :: new (file ! () , line ! () , column ! ())) ;
furnace_rs_core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_GRAPH_REPOSITORY }

#[test]
fn analysis_exposes_nodes_edges_and_a_dependency_ordered_plan() {
    let analysis = Furnace::builder().analyze();
    assert!(analysis.is_valid());
    assert_eq!(
        analysis
            .graph()
            .provider::<GraphDatabase>()
            .unwrap()
            .state(),
        ProviderState::Planned
    );
    assert_eq!(
        analysis
            .graph()
            .provider::<GraphRepository>()
            .unwrap()
            .origin(),
        ProviderOrigin::Provider
    );
    assert!(analysis.graph().dependencies().iter().any(|edge| {
        edge.provider_type_name().contains("GraphRepository")
            && edge.dependency_type_name().contains("GraphDatabase")
    }));
    assert_eq!(
        analysis
            .construction_plan()
            .unwrap()
            .steps()
            .iter()
            .map(ConstructionStep::type_name)
            .collect::<Vec<_>>(),
        ["GraphDatabase", "GraphRepository"],
    );
}
