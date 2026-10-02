//! Explicit application construction and lifecycle ownership.

use std::any::TypeId;
use std::collections::VecDeque;

use crate::auto_configuration::{
    self, AutoConfigurationApplyContext, AutoConfigurationDescriptor, AutoConfigurationInputs,
};
use crate::{
    ApplicationContext, ApplicationGraph, AutoConfigurationReport, Catalog, Cauldron,
    CauldronGraph, Config, ConstructionContext, ConstructionPlan, ConstructionStep, Diagnostic,
    Error, FURNACE006, FURNACE008, GraphAnalysis, LifecycleHook, LifecycleManager, LifecycleState,
    ProviderContribution, ProviderDescriptor, ProviderRegistry, Result,
    graph::{
        SatisfiedProvider, analyze_catalog, analyze_descriptors, build_cauldron_graph,
        select_focused_providers, select_scoped_providers,
    },
};

/// Builds an application by explicitly providing and constructing providers.
pub struct FurnaceBuilder {
    config: Config,
    registry: ProviderRegistry,
    satisfied: Vec<SatisfiedProvider>,
    auto_configuration_inputs: AutoConfigurationInputs,
    lifecycle: LifecycleManager,
    root: Option<CauldronRoot>,
    focus: Option<(TypeId, &'static str)>,
    required_supplies: Vec<TypeId>,
}

impl FurnaceBuilder {
    /// Creates a builder with configuration available to provider constructors and resolvers.
    pub fn new(config: Config) -> Self {
        let mut registry = ProviderRegistry::new();
        registry
            .insert(config.clone())
            .expect("a new provider registry cannot already contain configuration");

        Self {
            config,
            registry,
            satisfied: vec![SatisfiedProvider::provided::<Config>()],
            auto_configuration_inputs: AutoConfigurationInputs::default(),
            lifecycle: LifecycleManager::new(),
            root: None,
            focus: None,
            required_supplies: Vec::new(),
        }
    }

    /// Selects the root module for scoped analysis and construction.
    #[allow(clippy::result_large_err)]
    pub fn root<M: Cauldron>(&mut self) -> Result<&mut Self> {
        if let Some((_, name)) = self.focus {
            return Err(root_already_selected_error(
                name,
                std::any::type_name::<M>(),
            ));
        }
        if let Some(root) = &self.root {
            return Err(root_already_selected_error(
                root.type_name,
                std::any::type_name::<M>(),
            ));
        }
        self.root = Some(CauldronRoot::of::<M>());
        Ok(self)
    }

    /// Selects a registered dependency chain for an official test fixture.
    #[doc(hidden)]
    #[allow(clippy::result_large_err)]
    pub fn __test_focus<T: Send + Sync + 'static>(&mut self) -> Result<&mut Self> {
        if let Some(root) = &self.root {
            return Err(root_already_selected_error(
                root.type_name,
                std::any::type_name::<T>(),
            ));
        }
        if let Some((_, name)) = self.focus {
            return Err(root_already_selected_error(
                name,
                std::any::type_name::<T>(),
            ));
        }
        self.focus = Some((TypeId::of::<T>(), std::any::type_name::<T>()));
        Ok(self)
    }

    /// Requires a fixture-owned value rather than a registered constructor.
    #[doc(hidden)]
    pub fn __test_require_provided<T: Send + Sync + 'static>(&mut self) -> &mut Self {
        self.required_supplies.push(TypeId::of::<T>());
        self
    }

    /// Provides a concrete application-scoped value.
    #[allow(clippy::result_large_err)]
    pub fn provide<T>(&mut self, value: T) -> Result<&mut Self>
    where
        T: Send + Sync + 'static,
    {
        self.registry.insert(value)?;
        self.satisfied.push(SatisfiedProvider::provided::<T>());
        Ok(self)
    }

    /// Constructs exactly one statically declared provider using currently provided dependencies.
    #[allow(clippy::result_large_err)]
    pub async fn construct<T>(&mut self) -> Result<&mut Self>
    where
        T: Send + Sync + 'static,
    {
        let explicit = if let Some(root) = &self.root {
            let graph = build_cauldron_graph(root.type_id, &Catalog::cauldrons())?;
            graph.members().find_map(|(_, member)| {
                (member.type_id == TypeId::of::<T>())
                    .then_some(member.descriptor)
                    .flatten()
            })
        } else {
            None
        };
        let descriptor = match explicit {
            Some(descriptor) => descriptor,
            None => Catalog::provider_for::<T>()?,
        };
        let contribution = self.invoke_provider(descriptor).await?;
        self.apply_provider_contribution(descriptor, contribution)?;
        self.satisfied
            .push(SatisfiedProvider::preconstructed::<T>());
        Ok(self)
    }

    /// Analyzes the complete provider graph without invoking constructors.
    pub fn analyze(&self) -> GraphAnalysis {
        self.analyze_builder().public
    }

    /// Registers a private input for an official auto-configuration integration.
    #[doc(hidden)]
    pub fn __auto_configuration_input<T: Send + Sync + 'static>(
        &mut self,
        identifier: &'static str,
        input: T,
    ) -> bool {
        self.auto_configuration_inputs.insert(identifier, input)
    }

    /// Registers a hook that runs when the completed application starts and stops.
    pub fn lifecycle_hook<H>(&mut self, hook: H) -> &mut Self
    where
        H: LifecycleHook + 'static,
    {
        self.lifecycle.add_hook(hook);
        self
    }

    /// Registers a framework-owned infrastructure lifecycle hook.
    #[doc(hidden)]
    pub fn __infrastructure_lifecycle_hook<H>(&mut self, owner: &'static str, hook: H) -> &mut Self
    where
        H: LifecycleHook + 'static,
    {
        self.lifecycle.add_infrastructure_hook(owner, hook);
        self
    }

    /// Validates and automatically constructs the complete application graph.
    #[allow(clippy::result_large_err)]
    pub async fn build(mut self) -> Result<Furnace> {
        let BuilderAnalysis {
            public,
            selected,
            failure,
        } = self.analyze_builder();
        if !public.is_valid() {
            return Err(build_analysis_error(public, failure));
        }
        let (graph, construction_plan, auto_configurations, cauldron_graph) =
            public.into_valid_parts()?;

        for descriptor in selected {
            let context = AutoConfigurationApplyContext::new(
                descriptor.identifier(),
                &self.config,
                &self.auto_configuration_inputs,
                cauldron_graph.as_ref(),
            );
            let contribution = (descriptor.applier())(&context)?;
            let (provider, hooks) = contribution.into_parts();
            self.registry.insert_erased(
                descriptor.output_type_id(),
                descriptor.output_type_name(),
                provider,
            )?;
            for hook in hooks {
                self.lifecycle
                    .add_boxed_infrastructure_hook(descriptor.identifier(), hook);
            }
        }

        for step in construction_plan.steps() {
            let descriptor = step.descriptor();
            let contribution = self
                .invoke_provider(descriptor)
                .await
                .map_err(|source| provider_construction_error(step, &graph, source))?;
            self.apply_provider_contribution(descriptor, contribution)?;
        }

        Ok(Furnace {
            context: ApplicationContext::new(self.registry, self.config),
            lifecycle: self.lifecycle,
            graph,
            construction_plan,
            auto_configurations,
            cauldron_graph,
        })
    }

    fn analyze_builder(&self) -> BuilderAnalysis {
        let memo = crate::preflight::AnalysisMemo::default();
        let mut analysis = self.analyze_builder_parts(&memo);
        // A failed rooted selection has no virtual scope for integrations to inspect.
        if self.root.is_some() && analysis.public.cauldron_graph().is_none() {
            return analysis;
        }
        let context = crate::preflight::PreflightContext::new(
            &self.config,
            &analysis.public,
            self.focus.map(|(id, _)| id),
            &memo,
            &self.satisfied,
        );
        let diagnostics = crate::preflight::validate(&context);
        analysis.public.append_diagnostics(diagnostics);
        analysis
    }

    fn analyze_builder_parts(&self, memo: &crate::preflight::AnalysisMemo) -> BuilderAnalysis {
        let providers = Catalog::providers();
        if let Some((target, name)) = self.focus {
            return self.analyze_focused(target, name, &providers, memo);
        }
        let Some(root) = &self.root else {
            return self.analyze_complete_catalog(&providers, memo);
        };

        let cauldrons = Catalog::cauldrons();
        let mut cauldron_graph = match build_cauldron_graph(root.type_id, &cauldrons) {
            Ok(cauldron_graph) => cauldron_graph,
            Err(error) => {
                return BuilderAnalysis {
                    public: GraphAnalysis::invalid(error.diagnostics().to_vec()),
                    selected: Vec::new(),
                    failure: None,
                };
            }
        };
        let initial_scope = select_scoped_providers(&cauldron_graph, &providers, &self.satisfied);
        let auto_configuration = auto_configuration::analyze_parts_with_memo(
            &auto_configuration::descriptors(),
            &initial_scope.descriptors,
            &self.satisfied,
            &self.config,
            &self.auto_configuration_inputs,
            Some(&cauldron_graph),
            memo,
            None,
        );
        let mut satisfied = self.satisfied.clone();
        satisfied.extend(auto_configuration.virtual_satisfied);

        let scoped = select_scoped_providers(&cauldron_graph, &providers, &satisfied);
        let mut covered_missing = auto_configuration.covered_missing;
        for type_id in &scoped.covered_missing {
            if !covered_missing.contains(type_id) {
                covered_missing.push(*type_id);
            }
        }
        let mut public = analyze_descriptors(&scoped.descriptors, &satisfied, &covered_missing);
        public.prepend_diagnostics(scoped.diagnostics);
        public.append_diagnostics(auto_configuration.diagnostics);
        public.auto_configurations = auto_configuration.reports;
        cauldron_graph.set_provider_ownership(scoped.ownership);
        public.set_cauldron_graph(cauldron_graph);

        BuilderAnalysis {
            public,
            selected: auto_configuration.selected,
            failure: auto_configuration.failure,
        }
    }

    fn analyze_focused(
        &self,
        target: TypeId,
        name: &'static str,
        providers: &[&'static ProviderDescriptor],
        memo: &crate::preflight::AnalysisMemo,
    ) -> BuilderAnalysis {
        let initial = select_focused_providers(
            target,
            name,
            providers,
            &self.satisfied,
            &self.required_supplies,
            &[],
        );
        let selected_providers = providers
            .iter()
            .copied()
            .filter(|descriptor| {
                initial
                    .graph()
                    .providers
                    .iter()
                    .any(|node| node.type_id == descriptor.type_id())
            })
            .collect::<Vec<_>>();
        let automatic = auto_configuration::descriptors()
            .into_iter()
            .filter(|descriptor| {
                let output = descriptor.output_type_id();
                !self.required_supplies.contains(&output)
                    && (descriptor.focused_requirements()
                        || initial.graph().providers.iter().any(|node| {
                            node.type_id == output
                                || node
                                    .declared_dependencies
                                    .iter()
                                    .any(|dependency| dependency.type_id() == output)
                        }))
            })
            .collect::<Vec<_>>();
        let automatic = auto_configuration::analyze_parts_with_memo(
            &automatic,
            &selected_providers,
            &self.satisfied,
            &self.config,
            &self.auto_configuration_inputs,
            None,
            memo,
            self.focus.map(|(id, _)| id),
        );
        let mut satisfied = self.satisfied.clone();
        satisfied.extend(automatic.virtual_satisfied);
        let mut public = select_focused_providers(
            target,
            name,
            providers,
            &satisfied,
            &self.required_supplies,
            &automatic.covered_missing,
        );
        public.append_diagnostics(automatic.diagnostics);
        public.auto_configurations = automatic.reports;
        BuilderAnalysis {
            public,
            selected: automatic.selected,
            failure: automatic.failure,
        }
    }

    async fn invoke_provider(
        &self,
        descriptor: &'static ProviderDescriptor,
    ) -> Result<ProviderContribution> {
        let context = ConstructionContext::new(&self.registry, &self.config);
        if let Some(constructor) = descriptor.lifecycle_constructor() {
            constructor(&context).await
        } else {
            (descriptor.constructor())(&context)
                .await
                .map(ProviderContribution::from_provider)
        }
    }

    fn apply_provider_contribution(
        &mut self,
        descriptor: &'static ProviderDescriptor,
        contribution: ProviderContribution,
    ) -> Result<()> {
        let (provider, registrations) = contribution.into_parts();
        self.registry
            .insert_erased(descriptor.type_id(), descriptor.type_name(), provider)?;
        for registration in registrations {
            self.lifecycle.add_registration(registration);
        }
        Ok(())
    }

    fn analyze_complete_catalog(
        &self,
        providers: &[&'static crate::ProviderDescriptor],
        memo: &crate::preflight::AnalysisMemo,
    ) -> BuilderAnalysis {
        let auto_configuration = auto_configuration::analyze_parts_with_memo(
            &auto_configuration::descriptors(),
            providers,
            &self.satisfied,
            &self.config,
            &self.auto_configuration_inputs,
            None,
            memo,
            self.focus.map(|(id, _)| id),
        );
        let mut satisfied = self.satisfied.clone();
        satisfied.extend(auto_configuration.virtual_satisfied);

        let mut public = analyze_catalog(&satisfied, &auto_configuration.covered_missing);
        public.auto_configurations = auto_configuration.reports;
        public.append_diagnostics(auto_configuration.diagnostics);

        BuilderAnalysis {
            public,
            selected: auto_configuration.selected,
            failure: auto_configuration.failure,
        }
    }
}

struct CauldronRoot {
    type_id: TypeId,
    type_name: &'static str,
}

impl CauldronRoot {
    fn of<M: Cauldron>() -> Self {
        Self {
            type_id: TypeId::of::<M>(),
            type_name: std::any::type_name::<M>(),
        }
    }
}

struct BuilderAnalysis {
    public: GraphAnalysis,
    selected: Vec<&'static AutoConfigurationDescriptor>,
    failure: Option<Error>,
}

/// An explicitly constructed application and its lifecycle state.
pub struct Furnace {
    context: ApplicationContext,
    lifecycle: LifecycleManager,
    graph: ApplicationGraph,
    construction_plan: ConstructionPlan,
    auto_configurations: Vec<AutoConfigurationReport>,
    cauldron_graph: Option<CauldronGraph>,
}

impl Furnace {
    /// Creates a builder with empty configuration.
    pub fn builder() -> FurnaceBuilder {
        Self::builder_with_config(Config::empty())
    }

    /// Creates a builder with caller-supplied configuration.
    pub fn builder_with_config(config: Config) -> FurnaceBuilder {
        FurnaceBuilder::new(config)
    }

    /// Returns the application's current lifecycle state.
    pub const fn state(&self) -> LifecycleState {
        self.lifecycle.state()
    }

    /// Returns the immutable application context.
    pub const fn context(&self) -> &ApplicationContext {
        &self.context
    }

    /// Returns the immutable graph validated before construction.
    pub const fn graph(&self) -> &ApplicationGraph {
        &self.graph
    }

    /// Returns the deterministic provider construction plan that was executed.
    pub const fn construction_plan(&self) -> &ConstructionPlan {
        &self.construction_plan
    }

    /// Returns reports for official auto-configurations evaluated before the build.
    pub fn auto_configurations(&self) -> &[AutoConfigurationReport] {
        &self.auto_configurations
    }

    /// Returns the rooted module graph, or `None` for a complete-catalog build.
    pub const fn cauldron_graph(&self) -> Option<&CauldronGraph> {
        self.cauldron_graph.as_ref()
    }

    /// Starts registered lifecycle hooks.
    #[allow(clippy::result_large_err)]
    pub async fn start(&mut self) -> Result<()> {
        self.lifecycle.start(&self.context).await
    }

    /// Stops registered lifecycle hooks in reverse registration order.
    #[allow(clippy::result_large_err)]
    pub async fn shutdown(&mut self) -> Result<()> {
        self.lifecycle.shutdown(&self.context).await
    }
}

fn root_already_selected_error(existing: &'static str, attempted: &'static str) -> Error {
    Error::new(
        Diagnostic::new(
            FURNACE008,
            "application root already selected",
            format!("a builder already selected `{existing}` and cannot select another root"),
        )
        .with_subject(attempted),
    )
}

fn build_analysis_error(public: GraphAnalysis, failure: Option<Error>) -> Error {
    let GraphAnalysis { diagnostics, .. } = public;
    if let Some(failure) = failure {
        let primary = failure.diagnostic().clone();
        let mut primary_removed = false;
        let related = diagnostics.into_iter().filter(|diagnostic| {
            if !primary_removed && *diagnostic == primary {
                primary_removed = true;
                false
            } else {
                true
            }
        });
        return failure.with_related_diagnostics(related);
    }

    let mut diagnostics = diagnostics.into_iter();
    let primary = diagnostics
        .next()
        .expect("invalid analysis has diagnostics");
    Error::from_diagnostics(primary, diagnostics)
}

fn provider_construction_error(
    step: &ConstructionStep,
    graph: &ApplicationGraph,
    source: Error,
) -> Error {
    let mut diagnostic = Diagnostic::new(
        FURNACE006,
        "provider construction failed",
        "a provider constructor returned an error",
    )
    .with_subject(step.type_name())
    .with_location(step.location());
    if let Some(path) = consumer_path(graph, step) {
        diagnostic = diagnostic.with_suggestion(format!("construction path: {path}"));
    }
    Error::with_source(diagnostic, source)
}

fn consumer_path(graph: &ApplicationGraph, failing_step: &ConstructionStep) -> Option<String> {
    if !graph
        .dependencies
        .iter()
        .any(|edge| edge.dependency_type_id == failing_step.type_id())
    {
        return None;
    }

    let mut roots = graph
        .providers
        .iter()
        .enumerate()
        .filter(|(_, provider)| {
            !graph
                .dependencies
                .iter()
                .any(|edge| edge.dependency_type_id == provider.type_id)
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    roots.sort_by(|left, right| provider_order(graph, *left, *right));

    let mut visited = vec![false; graph.providers.len()];
    let mut queue = VecDeque::new();
    for root in roots {
        visited[root] = true;
        queue.push_back(vec![root]);
    }

    while let Some(path) = queue.pop_front() {
        let provider = *path.last().expect("paths are never empty");
        if graph.providers[provider].type_id == failing_step.type_id() {
            return Some(
                path.iter()
                    .map(|index| graph.providers[*index].type_name)
                    .collect::<Vec<_>>()
                    .join(" -> "),
            );
        }

        let mut dependencies = graph
            .dependencies
            .iter()
            .filter(|edge| edge.provider_type_id == graph.providers[provider].type_id)
            .filter_map(|edge| {
                graph
                    .providers
                    .iter()
                    .position(|candidate| candidate.type_id == edge.dependency_type_id)
            })
            .collect::<Vec<_>>();
        dependencies.sort_by(|left, right| provider_order(graph, *left, *right));
        for dependency in dependencies {
            if !visited[dependency] {
                visited[dependency] = true;
                let mut path = path.clone();
                path.push(dependency);
                queue.push_back(path);
            }
        }
    }

    None
}

fn provider_order(graph: &ApplicationGraph, left: usize, right: usize) -> std::cmp::Ordering {
    let left = &graph.providers[left];
    let right = &graph.providers[right];
    left.type_name
        .cmp(right.type_name)
        .then_with(|| left.origin.cmp(&right.origin))
        .then_with(|| match (left.location, right.location) {
            (Some(left), Some(right)) => left
                .file
                .cmp(right.file)
                .then_with(|| left.line.cmp(&right.line))
                .then_with(|| left.column.cmp(&right.column)),
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        })
}
