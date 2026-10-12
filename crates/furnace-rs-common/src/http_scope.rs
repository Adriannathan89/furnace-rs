//! HTTP metadata selected for one application build.

use std::any::TypeId;

use furnace_rs_core::{CauldronGraph, Furnace, Result};

use crate::{ControllerEndpointDescriptor, HttpMethod, RouteCatalog, RouteDescriptor};

#[cfg(feature = "jwt")]
use crate::GuardDescriptor;

/// A selected Passport guard and the module context that selected it.
#[cfg(feature = "jwt")]
#[derive(Clone)]
pub(crate) struct ScopedGuard {
    guard: &'static GuardDescriptor,
    occurrence: Option<RouteIdentity>,
    context_cauldron: Option<TypeId>,
}

#[cfg(feature = "jwt")]
impl ScopedGuard {
    pub(crate) const fn occurrence(&self) -> Option<RouteIdentity> {
        self.occurrence
    }
    pub(crate) const fn guard(&self) -> &'static GuardDescriptor {
        self.guard
    }

    pub(crate) const fn context_cauldron(&self) -> Option<TypeId> {
        self.context_cauldron
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
    passport_context_cauldron: Option<TypeId>,
}

impl RouteIdentity {
    fn new(controller: &ControllerEndpointDescriptor, route: &RouteDescriptor) -> Self {
        Self {
            controller: controller.type_id(),
            method: route.method(),
            full_path: route.full_path(),
            handler: route.handler(),
            #[cfg(feature = "jwt")]
            passport_context_cauldron: None,
        }
    }

    #[cfg(feature = "jwt")]
    fn with_passport_context_cauldron(mut self, context_cauldron: Option<TypeId>) -> Self {
        self.passport_context_cauldron = context_cauldron;
        self
    }

    pub(crate) fn matches(
        &self,
        controller: &ControllerEndpointDescriptor,
        route: &RouteDescriptor,
    ) -> bool {
        self.controller == controller.type_id()
            && self.method == route.method()
            && self.full_path == route.full_path()
            && self.handler == route.handler()
    }

    #[cfg(feature = "jwt")]
    const fn passport_context_cauldron(&self) -> Option<TypeId> {
        self.passport_context_cauldron
    }
}

/// A controller selected for one HTTP application, including its selected routes.
#[derive(Clone)]
pub(crate) struct ScopedController {
    #[cfg(feature = "jwt")]
    seal: Option<ControllerSeal>,
    descriptor: &'static ControllerEndpointDescriptor,
    selected_routes: Vec<RouteIdentity>,
    context_cauldron: Option<TypeId>,
}

impl ScopedController {
    pub(crate) const fn descriptor(&self) -> &'static ControllerEndpointDescriptor {
        self.descriptor
    }

    pub(crate) fn selects(&self, route: &RouteDescriptor) -> bool {
        self.selected_routes
            .iter()
            .any(|identity| identity.matches(self.descriptor, route))
    }

    #[cfg(feature = "jwt")]
    pub(crate) fn passport_context_cauldron(&self, route: &RouteDescriptor) -> Option<TypeId> {
        self.selected_routes
            .iter()
            .find(|identity| identity.matches(self.descriptor, route))
            .and_then(RouteIdentity::passport_context_cauldron)
    }

    #[cfg(feature = "jwt")]
    pub(crate) fn sealed_endpoint(&self, route: &RouteDescriptor) -> Option<RouteIdentity> {
        self.guard_active(route)
            .then(|| {
                self.selected_routes
                    .iter()
                    .find(|identity| identity.matches(self.descriptor, route))
                    .copied()
            })
            .flatten()
    }
    pub(crate) fn guard_active(&self, route: &RouteDescriptor) -> bool {
        #[cfg(feature = "jwt")]
        {
            self.seal.is_some() && !route.seal_skipped()
        }
        #[cfg(not(feature = "jwt"))]
        {
            let _ = route;
            false
        }
    }

    fn has_routes(&self) -> bool {
        !self.selected_routes.is_empty()
    }

    #[allow(dead_code)]
    pub(crate) const fn context_cauldron(&self) -> Option<TypeId> {
        self.context_cauldron
    }
}

/// The HTTP controller, route, and guard metadata selected for one application.
#[derive(Clone)]
pub(crate) struct HttpApplicationScope {
    controllers: Vec<ScopedController>,
    #[cfg(feature = "jwt")]
    guards: Vec<ScopedGuard>,
}

impl HttpApplicationScope {
    pub(crate) fn for_test_controller<T: Send + Sync + 'static>() -> Result<Self> {
        let matches = RouteCatalog::route_controllers(|id| id == TypeId::of::<T>())?
            .into_iter()
            .filter(|descriptor| descriptor.type_id() == TypeId::of::<T>())
            .collect::<Vec<_>>();
        let descriptor = match matches.as_slice() {
            [descriptor] => *descriptor,
            _ => {
                return Err(furnace_rs_core::Error::new(
                    furnace_rs_core::Diagnostic::new(
                        furnace_rs_core::FURNACE030,
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
            #[cfg(feature = "jwt")]
            seal: None,
            descriptor,
            selected_routes: descriptor
                .routes()
                .map(|route| RouteIdentity::new(descriptor, route))
                .collect(),
            context_cauldron: None,
        }];
        Self::finish(controllers, None, true)
    }

    pub(crate) fn for_focus(target: TypeId) -> Result<Self> {
        let controllers = RouteCatalog::route_controllers(|id| id == target)?
            .into_iter()
            .map(|descriptor| ScopedController {
                #[cfg(feature = "jwt")]
                seal: None,
                descriptor,
                selected_routes: descriptor
                    .routes()
                    .map(|route| RouteIdentity::new(descriptor, route))
                    .collect(),
                context_cauldron: None,
            })
            .collect();
        Self::finish(controllers, None, true)
    }

    fn finish(
        mut controllers: Vec<ScopedController>,
        graph: Option<&CauldronGraph>,
        focused: bool,
    ) -> Result<Self> {
        #[cfg(feature = "jwt")]
        let mut guards = Vec::new();
        let _ = (graph, focused);
        let declarations = RouteCatalog::controllers();
        for controller in &mut controllers {
            if let Some(declaration) = declarations
                .iter()
                .find(|entry| entry.type_id() == controller.descriptor.type_id())
            {
                let definition = declaration.seals();
                #[cfg(feature = "jwt")]
                {
                    if definition.entries().len() > 1 {
                        return Err(furnace_rs_core::Error::new(
                            furnace_rs_core::Diagnostic::new(
                                furnace_rs_core::FURNACE008,
                                "multiple controller seals",
                                "a controller can declare at most one guard policy",
                            )
                            .with_subject(declaration.type_name())
                            .with_location(definition.entries()[1].location()),
                        ));
                    }
                    if let Some(entry) = definition.entries().first() {
                        let seal = ControllerSeal {
                            guard: entry.descriptor(),
                            location: entry.location(),
                            policy_type_id: entry.guard_type_id(),
                            policy_type_name: entry.guard_type_name(),
                        };
                        for occurrence in controller.selected_routes.iter().filter(|identity| {
                            controller.descriptor.routes().any(|route| {
                                identity.matches(controller.descriptor, route)
                                    && !route.seal_skipped()
                            })
                        }) {
                            guards.push(ScopedGuard {
                                guard: seal.guard,
                                occurrence: Some(*occurrence),
                                context_cauldron: controller.context_cauldron,
                            });
                        }
                        controller.seal = Some(seal);
                    }
                }
                #[cfg(not(feature = "jwt"))]
                let _ = definition;
            }
        }
        Ok(Self {
            controllers,
            #[cfg(feature = "jwt")]
            guards,
        })
    }

    pub(crate) fn for_application(application: &Furnace) -> Result<Self> {
        Self::for_cauldron_graph(application.cauldron_graph())
    }

    pub(crate) fn for_cauldron_graph(cauldron_graph: Option<&CauldronGraph>) -> Result<Self> {
        if let Some(graph) = cauldron_graph {
            for output in graph.registered_controllers() {
                if !RouteCatalog::route_controllers(|id| id == output)?
                    .iter()
                    .any(|controller| controller.type_id() == output)
                {
                    return Err(furnace_rs_core::Error::new(
                        furnace_rs_core::Diagnostic::new(
                            furnace_rs_core::FURNACE008,
                            "missing registered controller",
                            "a controller registration has no controller metadata",
                        ),
                    ));
                }
            }
        }
        let controllers = match cauldron_graph {
            None => Self::complete_controllers()?,
            Some(graph) => Self::rooted_controllers(graph)?,
        };

        Self::finish(controllers, cauldron_graph, false)
    }

    /// Creates an inspection-only scope rooted in a successfully analyzed module graph.
    ///
    /// Unlike complete-catalog runtime selection, missing rooted analysis deliberately
    /// produces an empty scope so inspection cannot expose unrelated linked metadata.
    #[allow(dead_code)] // Used by the private inspection path added in the next task.
    pub(crate) fn for_rooted_inspection(cauldron_graph: Option<&CauldronGraph>) -> Result<Self> {
        let controllers = match cauldron_graph {
            Some(graph) => Self::rooted_controllers(graph)?,
            None => Vec::new(),
        };
        Self::finish(controllers, cauldron_graph, false)
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
    ) -> impl Iterator<Item = (&ScopedController, &RouteDescriptor)> {
        self.controllers.iter().flat_map(|controller| {
            controller
                .descriptor()
                .routes()
                .filter(move |route| controller.selects(route))
                .map(move |route| (controller, route))
        })
    }

    #[cfg(feature = "jwt")]
    pub(crate) fn guards(&self) -> &[ScopedGuard] {
        &self.guards
    }

    fn complete_controllers() -> Result<Vec<ScopedController>> {
        Ok(RouteCatalog::route_controllers(|_| true)?
            .into_iter()
            .map(|descriptor| ScopedController {
                #[cfg(feature = "jwt")]
                seal: None,
                descriptor,
                selected_routes: descriptor
                    .routes()
                    .map(|route| RouteIdentity::new(descriptor, route))
                    .collect(),
                context_cauldron: None,
            })
            .collect())
    }

    fn rooted_controllers(graph: &CauldronGraph) -> Result<Vec<ScopedController>> {
        Ok(
            RouteCatalog::route_controllers(|id| graph.is_controller(id))?
                .into_iter()
                .filter(|descriptor| graph.is_controller(descriptor.type_id()))
                .map(|descriptor| {
                    let context_cauldron = graph
                        .owner_of(descriptor.type_id())
                        .map(|owner| owner.type_id());
                    let selected_routes = descriptor
                        .routes()
                        .map(|route| {
                            let identity = RouteIdentity::new(descriptor, route);
                            #[cfg(feature = "jwt")]
                            let identity =
                                identity.with_passport_context_cauldron(context_cauldron);
                            identity
                        })
                        .collect();
                    ScopedController {
                        #[cfg(feature = "jwt")]
                        seal: None,
                        descriptor,
                        selected_routes,
                        context_cauldron,
                    }
                })
                .collect(),
        )
    }
}

/// Static policy metadata belongs to an occurrence in its controller's context.
#[cfg(feature = "jwt")]
#[derive(Clone, Copy)]
#[allow(dead_code)] // Fields are consumed by endpoint middleware/inspection in the following tasks.
pub(crate) struct ControllerSeal {
    pub(crate) guard: &'static GuardDescriptor,
    pub(crate) location: furnace_rs_core::SourceLocation,
    pub(crate) policy_type_id: TypeId,
    pub(crate) policy_type_name: &'static str,
}
