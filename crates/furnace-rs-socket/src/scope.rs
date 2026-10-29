//! Root-selected WebSocket validation with no provider construction.
use crate::{
    EndpointMode, ProcessingMode, SocketEndpointDescriptor, descriptor::SocketEndpointRegistration,
};
use furnace_rs_core::{CauldronGraph, Furnace, Result};
use std::collections::BTreeSet;

/// Selected, validated application endpoints.
pub struct SocketApplicationScope {
    endpoints: Vec<&'static SocketEndpointDescriptor>,
}

impl SocketApplicationScope {
    /// Selects endpoint providers from the explicit cauldron graph.
    #[allow(clippy::result_large_err)]
    pub fn for_cauldron_graph(graph: Option<&CauldronGraph>) -> Result<Self> {
        let mut endpoints =
            furnace_rs_core::__private::inventory::iter::<SocketEndpointRegistration>
                .into_iter()
                .map(|r| r.descriptor)
                .collect::<Vec<_>>();

        if let Some(graph) = graph {
            let selected = graph
                .registered_transport_providers("socket")
                .collect::<BTreeSet<_>>();
            endpoints.retain(|e| selected.contains(&(e.provider)()));

            if selected
                .iter()
                .any(|id| !endpoints.iter().any(|e| (e.provider)() == *id))
            {
                return Err(crate::config::invalid("socket.registration"));
            }
        }

        endpoints.sort_by_key(|e| e.path);
        Self::validate_descriptors(&endpoints)?;

        Ok(Self { endpoints })
    }

    /// Selects endpoints reachable from the built application's root.
    #[allow(clippy::result_large_err)]
    pub fn for_application(app: &Furnace) -> Result<Self> {
        Self::for_cauldron_graph(app.cauldron_graph())
    }

    /// Validates metadata without resolving application dependencies.
    #[allow(clippy::result_large_err)]
    pub fn validate_descriptors(endpoints: &[&SocketEndpointDescriptor]) -> Result<()> {
        let mut paths = BTreeSet::new();

        for e in endpoints {
            if e.cookie.is_empty()
                || e.query.is_empty()
                || !e
                    .cookie
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
            {
                return Err(crate::config::invalid("socket.credentials"));
            }

            if !e.path.starts_with('/')
                || e.path.contains(['{', '}', '*', '?', '#'])
                || !paths.insert(e.path)
            {
                return Err(crate::config::invalid("socket.paths"));
            }

            if matches!(
                e.processing,
                ProcessingMode::Concurrent { max_concurrency: 0 }
            ) {
                return Err(crate::config::invalid("socket.processing"));
            }

            let mut events = BTreeSet::new();
            let mut kinds = BTreeSet::new();

            for h in e.handlers {
                if h.kind == "event" {
                    if h.name.is_empty()
                        || h.name.starts_with("furnace.")
                        || !events.insert(h.name)
                        || e.mode != EndpointMode::Events
                    {
                        return Err(crate::config::invalid("socket.events"));
                    }
                } else if !kinds.insert(h.kind)
                    || (matches!(h.kind, "text" | "binary") && e.mode != EndpointMode::Raw)
                {
                    return Err(crate::config::invalid("socket.handlers"));
                }
            }
        }

        Ok(())
    }

    /// Endpoints in path order.
    pub fn endpoints(&self) -> &[&'static SocketEndpointDescriptor] {
        &self.endpoints
    }
}
