//! HTTP metadata selected for one application build.

use std::any::TypeId;

use mads_core::{Mads, ModuleGraph, Result};

use crate::{
    ControllerRouteDescriptor, HttpMethod, RouteCatalog, RouteContractDescriptor, RouteDescriptor,
};

#[cfg(feature = "jwt")]
use crate::{GuardCatalog, GuardDescriptor};

/// A selected Passport guard and the module context that selected it.
#[cfg(feature = "jwt")]
pub(crate) struct ScopedGuard {
    guard: &'static GuardDescriptor,
    context_module: Option<TypeId>,
}

#[cfg(feature = "jwt")]
impl ScopedGuard {
    pub(crate) const fn guard(&self) -> &'static GuardDescriptor {
        self.guard
    }

    pub(crate) const fn context_module(&self) -> Option<TypeId> {
        self.context_module
    }
}

/// A static identity for one route emitted by one controller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RouteIdentity {
    controller: TypeId,
    method: HttpMethod,
    full_path: &'static str,
    handler: &'static str,
    #[cfg(feature = "jwt")]
    passport_context_module: Option<TypeId>,
}

impl RouteIdentity {
    fn new(controller: &ControllerRouteDescriptor, route: &RouteDescriptor) -> Self {
        Self {
            controller: controller.type_id(),
            method: route.method(),
            full_path: route.full_path(),
            handler: route.handler(),
            #[cfg(feature = "jwt")]
            passport_context_module: None,
        }
    }

    #[cfg(feature = "jwt")]
    fn with_passport_context_module(mut self, context_module: Option<TypeId>) -> Self {
        self.passport_context_module = context_module;
        self
    }

    pub(crate) fn matches(
        &self,
        controller: &ControllerRouteDescriptor,
        route: &RouteDescriptor,
    ) -> bool {
        self.controller == controller.type_id()
            && self.method == route.method()
            && self.full_path == route.full_path()
            && self.handler == route.handler()
    }

    #[cfg(feature = "jwt")]
    const fn passport_context_module(&self) -> Option<TypeId> {
        self.passport_context_module
    }
}

/// A controller selected for one HTTP application, including its selected routes.
pub(crate) struct ScopedController {
    descriptor: &'static ControllerRouteDescriptor,
    selected_routes: Vec<RouteIdentity>,
    context_module: Option<TypeId>,
}

impl ScopedController {
    pub(crate) const fn descriptor(&self) -> &'static ControllerRouteDescriptor {
        self.descriptor
    }

    pub(crate) fn selects(&self, route: &RouteDescriptor) -> bool {
        self.selected_routes
            .iter()
            .any(|identity| identity.matches(self.descriptor, route))
    }

    #[cfg(feature = "jwt")]
    pub(crate) fn passport_context_module(&self, route: &RouteDescriptor) -> Option<TypeId> {
        self.selected_routes
            .iter()
            .find(|identity| identity.matches(self.descriptor, route))
            .and_then(RouteIdentity::passport_context_module)
    }

    fn has_routes(&self) -> bool {
        !self.selected_routes.is_empty()
    }

    #[allow(dead_code)]
    pub(crate) const fn context_module(&self) -> Option<TypeId> {
        self.context_module
    }
}

/// The HTTP controller, route, and guard metadata selected for one application.
pub(crate) struct HttpApplicationScope {
    controllers: Vec<ScopedController>,
    #[cfg(feature = "jwt")]
    guards: Vec<ScopedGuard>,
}

impl HttpApplicationScope {
    pub(crate) fn for_test_controller<T: Send + Sync + 'static>() -> Result<Self> {
        let matches = RouteCatalog::controllers()
            .into_iter()
            .filter(|descriptor| descriptor.type_id() == TypeId::of::<T>())
            .collect::<Vec<_>>();
        let descriptor = match matches.as_slice() {
            [descriptor] => *descriptor,
            _ => {
                return Err(mads_core::Error::new(
                    mads_core::Diagnostic::new(
                        mads_core::MADS030,
                        "invalid test controller",
                        if matches.is_empty() {
                            "selected controller has no route metadata"
                        } else {
                            "selected controller has ambiguous route metadata"
                        },
                    )
                    .with_subject(std::any::type_name::<T>()),
                ));
            }
        };
        let controllers = vec![ScopedController {
            descriptor,
            selected_routes: descriptor
                .contracts()
                .iter()
                .flat_map(|contract| contract.routes())
                .map(|route| RouteIdentity::new(descriptor, route))
                .collect(),
            context_module: None,
        }];
        #[cfg(feature = "jwt")]
        let guards = controllers
            .iter()
            .flat_map(|controller| {
                controller
                    .descriptor()
                    .contracts()
                    .iter()
                    .flat_map(|contract| contract.routes())
                    .filter_map(|route| {
                        route.guard().map(|guard| ScopedGuard {
                            guard,
                            context_module: None,
                        })
                    })
            })
            .collect();
        Ok(Self {
            controllers,
            #[cfg(feature = "jwt")]
            guards,
        })
    }

    pub(crate) fn for_application(application: &Mads) -> Result<Self> {
        Self::for_module_graph(application.module_graph())
    }

    pub(crate) fn for_module_graph(module_graph: Option<&ModuleGraph>) -> Result<Self> {
        if let Some(graph) = module_graph {
            for output in graph.registered_controllers() {
                if !RouteCatalog::controllers()
                    .iter()
                    .any(|controller| controller.type_id() == output)
                {
                    return Err(mads_core::Error::new(mads_core::Diagnostic::new(
                        mads_core::MADS008,
                        "missing registered controller",
                        "a controller registration has no controller metadata",
                    )));
                }
            }
        }
        let controllers = match module_graph {
            None => Self::complete_controllers(),
            Some(graph) => Self::rooted_controllers(graph),
        };

        #[cfg(feature = "jwt")]
        let guards = Self::selected_guards(module_graph, &controllers);

        Ok(Self {
            controllers,
            #[cfg(feature = "jwt")]
            guards,
        })
    }

    /// Creates an inspection-only scope rooted in a successfully analyzed module graph.
    ///
    /// Unlike complete-catalog runtime selection, missing rooted analysis deliberately
    /// produces an empty scope so inspection cannot expose unrelated linked metadata.
    #[allow(dead_code)] // Used by the private inspection path added in the next task.
    pub(crate) fn for_rooted_inspection(module_graph: Option<&ModuleGraph>) -> Result<Self> {
        let controllers = module_graph
            .map(Self::rooted_controllers)
            .unwrap_or_default();
        #[cfg(feature = "jwt")]
        let guards = match module_graph {
            Some(graph) => Self::selected_guards(Some(graph), &controllers),
            None => Vec::new(),
        };
        Ok(Self {
            controllers,
            #[cfg(feature = "jwt")]
            guards,
        })
    }

    pub(crate) fn controllers(&self) -> &[ScopedController] {
        &self.controllers
    }

    pub(crate) fn has_routes(&self) -> bool {
        self.controllers.iter().any(ScopedController::has_routes)
    }

    /// Iterates selected static route metadata without constructing controllers.
    #[allow(dead_code)] // Used by the private inspection path added in the next task.
    pub(crate) fn route_records(
        &self,
    ) -> impl Iterator<
        Item = (
            &ControllerRouteDescriptor,
            &RouteContractDescriptor,
            &RouteDescriptor,
        ),
    > {
        self.controllers.iter().flat_map(|controller| {
            controller
                .descriptor()
                .contracts()
                .iter()
                .flat_map(move |contract| {
                    contract
                        .routes()
                        .iter()
                        .filter(move |route| controller.selects(route))
                        .map(move |route| (controller.descriptor(), contract, route))
                })
        })
    }

    #[cfg(feature = "jwt")]
    pub(crate) fn guards(&self) -> &[ScopedGuard] {
        &self.guards
    }

    fn complete_controllers() -> Vec<ScopedController> {
        RouteCatalog::controllers()
            .into_iter()
            .map(|descriptor| ScopedController {
                descriptor,
                selected_routes: descriptor
                    .contracts()
                    .iter()
                    .flat_map(|contract| contract.routes())
                    .map(|route| RouteIdentity::new(descriptor, route))
                    .collect(),
                context_module: None,
            })
            .collect()
    }

    fn rooted_controllers(graph: &ModuleGraph) -> Vec<ScopedController> {
        RouteCatalog::controllers()
            .into_iter()
            .filter(|descriptor| graph.is_controller(descriptor.type_id()))
            .map(|descriptor| {
                let context_module = graph
                    .owner_of(descriptor.type_id())
                    .map(|owner| owner.type_id());
                let selected_routes = descriptor
                    .contracts()
                    .iter()
                    .flat_map(|contract| contract.routes())
                    .map(|route| {
                        let identity = RouteIdentity::new(descriptor, route);
                        #[cfg(feature = "jwt")]
                        let identity = identity.with_passport_context_module(context_module);
                        identity
                    })
                    .collect();
                ScopedController {
                    descriptor,
                    selected_routes,
                    context_module,
                }
            })
            .collect()
    }

    #[cfg(feature = "jwt")]
    fn selected_guards(
        module_graph: Option<&ModuleGraph>,
        controllers: &[ScopedController],
    ) -> Vec<ScopedGuard> {
        if module_graph.is_none() {
            return GuardCatalog::guards()
                .into_iter()
                .map(|guard| ScopedGuard {
                    guard,
                    context_module: None,
                })
                .collect();
        };

        controllers
            .iter()
            .flat_map(|controller| {
                controller
                    .descriptor()
                    .contracts()
                    .iter()
                    .flat_map(|contract| contract.routes())
                    .filter(move |route| controller.selects(route))
                    .filter_map(move |route| {
                        route.guard().map(|guard| ScopedGuard {
                            guard,
                            context_module: controller.passport_context_module(route),
                        })
                    })
            })
            .collect()
    }
}
