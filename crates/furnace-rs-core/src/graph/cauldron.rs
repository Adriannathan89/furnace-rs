//! Immutable module graph records and deterministic rooted traversal.

use std::any::TypeId;
use std::collections::{HashMap, HashSet};

use crate::{CauldronDescriptor, Diagnostic, Error, FURNACE008, Result, SourceLocation};

/// A reachable module in a rooted application graph.
pub struct CauldronNode {
    type_id: TypeId,
    type_name: &'static str,
    namespace: &'static str,
    location: SourceLocation,
    definition: Option<crate::CauldronDefinition>,
}

impl CauldronNode {
    /// Returns the module's runtime type identifier.
    pub const fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Returns the module's stable Rust type name.
    pub const fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Returns the Rust namespace containing the cauldron declaration.
    pub const fn namespace(&self) -> &'static str {
        self.namespace
    }

    /// Returns the module declaration's source location.
    pub const fn location(&self) -> SourceLocation {
        self.location
    }
}

/// A direct, authored import between two reachable cauldrons.
pub struct CauldronImportEdge {
    importer: usize,
    imported: usize,
}

impl CauldronImportEdge {
    /// Returns the module declaring this import.
    pub fn importer<'a>(&self, graph: &'a CauldronGraph) -> &'a CauldronNode {
        &graph.cauldrons[self.importer]
    }

    /// Returns the directly imported module.
    pub fn imported<'a>(&self, graph: &'a CauldronGraph) -> &'a CauldronNode {
        &graph.cauldrons[self.imported]
    }
}

/// Associates a selected provider with its owning reachable module, if any.
pub struct ProviderOwnership {
    pub(crate) provider_type_name: &'static str,
    pub(crate) cauldron_type_name: Option<&'static str>,
}

impl ProviderOwnership {
    /// Returns the selected provider's stable type name.
    pub const fn provider_type_name(&self) -> &'static str {
        self.provider_type_name
    }

    /// Returns the owning module type name, or `None` for an unowned provider.
    pub const fn cauldron_type_name(&self) -> Option<&'static str> {
        self.cauldron_type_name
    }
}

/// The deterministic module scope retained for a rooted application.
pub struct CauldronGraph {
    root: usize,
    cauldrons: Vec<CauldronNode>,
    imports: Vec<CauldronImportEdge>,
    provider_ownership: Vec<ProviderOwnership>,
    lookup: Lookup,
}

/// Constant-time views over the finished module graph.
#[derive(Default)]
struct Lookup {
    nodes: HashMap<TypeId, usize>,
    owners: HashMap<TypeId, usize>,
    controllers: HashSet<TypeId>,
    direct_imports: HashSet<(TypeId, TypeId)>,
}

impl Lookup {
    fn build(cauldrons: &[CauldronNode], imports: &[CauldronImportEdge]) -> Self {
        let mut lookup = Self::default();

        for (index, node) in cauldrons.iter().enumerate() {
            lookup.nodes.insert(node.type_id, index);

            for member in node
                .definition
                .iter()
                .flat_map(|definition| &definition.members)
            {
                lookup.owners.entry(member.type_id).or_insert(index);

                if member.controller {
                    lookup.controllers.insert(member.type_id);
                }
            }
        }

        for edge in imports {
            lookup.direct_imports.insert((
                cauldrons[edge.importer].type_id,
                cauldrons[edge.imported].type_id,
            ));
        }

        lookup
    }
}

impl CauldronGraph {
    /// Returns the selected root module.
    pub fn root(&self) -> &CauldronNode {
        &self.cauldrons[self.root]
    }

    /// Returns reachable cauldrons in deterministic declaration-order traversal.
    pub fn cauldrons(&self) -> &[CauldronNode] {
        &self.cauldrons
    }

    /// Returns every direct import edge in deterministic traversal order.
    pub fn imports(&self) -> &[CauldronImportEdge] {
        &self.imports
    }

    /// Returns selected provider ownership records ordered by provider type name.
    pub fn provider_ownership(&self) -> &[ProviderOwnership] {
        &self.provider_ownership
    }

    /// Returns the explicit owner of a registered output.
    pub fn owner_of(&self, output: TypeId) -> Option<&CauldronNode> {
        self.lookup
            .owners
            .get(&output)
            .map(|&index| &self.cauldrons[index])
    }

    /// Returns whether a cauldron explicitly exports an output.
    pub fn exports(&self, module: TypeId, output: TypeId) -> bool {
        self.node(module)
            .and_then(|node| node.definition.as_ref())
            .is_some_and(|definition| {
                definition
                    .exports
                    .iter()
                    .any(|member| member.type_id == output)
            })
    }

    /// Returns whether a reachable cauldron is global.
    pub fn is_global(&self, module: TypeId) -> bool {
        self.node(module)
            .and_then(|node| node.definition.as_ref())
            .is_some_and(|definition| definition.global)
    }

    /// Returns whether the output was explicitly registered as a controller.
    pub fn is_controller(&self, output: TypeId) -> bool {
        self.lookup.controllers.contains(&output)
    }

    /// Returns whether a cauldron can consume a registered output.
    pub fn can_access(&self, requester: TypeId, output: TypeId) -> bool {
        self.owner_of(output).is_some_and(|owner| {
            owner.type_id == requester
                || self.exports(owner.type_id, output)
                    && (self.directly_imports(requester, owner.type_id)
                        || self.is_global(owner.type_id))
        })
    }

    /// Iterates explicitly registered controller output identifiers.
    #[doc(hidden)]
    pub fn registered_controllers(&self) -> impl Iterator<Item = TypeId> + '_ {
        self.members()
            .filter(|(_, member)| member.controller)
            .map(|(_, member)| member.type_id)
    }

    /// Iterates explicitly selected service outputs belonging to one transport.
    #[doc(hidden)]
    pub fn registered_transport_providers<'a>(
        &'a self,
        transport: &'a str,
    ) -> impl Iterator<Item = TypeId> + 'a {
        self.members()
            .filter(move |(_, member)| member.transport == Some(transport))
            .map(|(_, member)| member.type_id)
    }

    /// Iterates members paired with their owning cauldron in traversal order.
    pub(crate) fn members(
        &self,
    ) -> impl Iterator<Item = (&CauldronNode, &crate::cauldron::CauldronMember)> {
        self.cauldrons.iter().flat_map(|node| {
            node.definition.iter().flat_map(move |definition| {
                definition.members.iter().map(move |member| (node, member))
            })
        })
    }

    /// Returns whether `importer` directly imports `imported`.
    #[doc(hidden)]
    pub fn directly_imports(&self, importer: TypeId, imported: TypeId) -> bool {
        self.lookup.direct_imports.contains(&(importer, imported))
    }

    /// Finds a reachable module node by its concrete type.
    fn node(&self, module: TypeId) -> Option<&CauldronNode> {
        self.lookup
            .nodes
            .get(&module)
            .map(|&index| &self.cauldrons[index])
    }

    /// Replaces ownership records after provider selection has been validated.
    pub(crate) fn set_provider_ownership(&mut self, ownership: Vec<ProviderOwnership>) {
        self.provider_ownership = ownership;
    }
}

/// Resolves the root's direct-import closure without constructing application providers.
pub(crate) fn build_cauldron_graph(
    root_type_id: TypeId,
    cauldrons: &[&'static CauldronDescriptor],
) -> Result<CauldronGraph> {
    let mut by_type: HashMap<TypeId, Vec<&'static CauldronDescriptor>> = HashMap::new();

    for &descriptor in cauldrons {
        by_type
            .entry(descriptor.type_id())
            .or_default()
            .push(descriptor);
    }

    let mut state = BuildState {
        by_type,
        claimed_members: HashSet::new(),
        node_ids: HashMap::new(),
        visited: HashSet::new(),
        stack: Vec::new(),
        cauldrons: Vec::new(),
        imports: Vec::new(),
    };
    visit(root_type_id, &mut state)?;
    let root = state
        .node_index(root_type_id)
        .expect("successful traversal records the root module");

    let lookup = Lookup::build(&state.cauldrons, &state.imports);

    Ok(CauldronGraph {
        root,
        cauldrons: state.cauldrons,
        imports: state.imports,
        provider_ownership: Vec::new(),
        lookup,
    })
}

/// Visits one module, validates its local declaration, and recursively follows imports.
fn visit(module: TypeId, state: &mut BuildState) -> Result<()> {
    if let Some(cycle_start) = state.stack.iter().position(|id| *id == module) {
        return Err(state.module_cycle_error(&state.stack[cycle_start..], module));
    }

    if !state.visited.insert(module) {
        return Ok(());
    }

    let descriptor = state.require_descriptor(module)?;
    let node = state.push_node(descriptor)?;
    state.stack.push(module);

    if let Some(register) = descriptor.registration() {
        let definition = register();
        let mut seen = HashSet::new();

        for member in &definition.members {
            if !seen.insert(member.type_id) || state.claimed_members.contains(&member.type_id) {
                return Err(registration_error(
                    "duplicate cauldron member",
                    member.type_name,
                    member.location,
                    descriptor.type_name(),
                ));
            }
        }

        state.claimed_members.extend(seen);

        let mut exports = HashSet::new();

        for export in &definition.exports {
            if !exports.insert(export.type_id)
                || !definition
                    .members
                    .iter()
                    .any(|member| member.type_id == export.type_id && !member.controller)
            {
                return Err(registration_error(
                    "invalid cauldron export",
                    export.type_name,
                    export.location,
                    descriptor.type_name(),
                ));
            }
        }

        let mut imports = HashSet::new();

        for import in &definition.imports {
            if !imports.insert(import.type_id) {
                return Err(registration_error(
                    "duplicate direct cauldron import",
                    import.type_name,
                    import.location,
                    descriptor.type_name(),
                ));
            }
        }

        let imports: Vec<_> = definition
            .imports
            .iter()
            .map(|import| import.type_id)
            .collect();
        state.cauldrons[node].definition = Some(definition);

        for imported in imports {
            let imported_descriptor = state.require_descriptor(imported)?;

            if imported_descriptor.registration().is_none() {
                return Err(registration_error(
                    "missing cauldron registration",
                    imported_descriptor.type_name(),
                    imported_descriptor.location(),
                    descriptor.type_name(),
                ));
            }

            let imported_node = state.ensure_node(imported_descriptor)?;
            state.push_edge(node, imported_node);
            visit(imported, state)?;
        }
    } else {
        return Err(registration_error(
            "missing cauldron registration",
            descriptor.type_name(),
            descriptor.location(),
            descriptor.type_name(),
        ));
    }

    state.stack.pop();

    Ok(())
}

struct BuildState {
    /// Static declarations grouped by module type, in catalog order.
    by_type: HashMap<TypeId, Vec<&'static CauldronDescriptor>>,
    /// Members already registered by earlier modules.
    claimed_members: HashSet<TypeId>,
    node_ids: HashMap<TypeId, usize>,
    visited: HashSet<TypeId>,
    stack: Vec<TypeId>,
    cauldrons: Vec<CauldronNode>,
    imports: Vec<CauldronImportEdge>,
}

impl BuildState {
    /// Resolves one module type to exactly one static declaration.
    fn require_descriptor(&self, type_id: TypeId) -> Result<&'static CauldronDescriptor> {
        let mut matches = self.by_type.get(&type_id).cloned().unwrap_or_default();
        matches.sort_by(|left, right| {
            left.type_name()
                .cmp(right.type_name())
                .then_with(|| left.location().cmp(&right.location()))
        });

        match matches.as_slice() {
            [] => Err(Error::new(
                Diagnostic::new(
                    FURNACE008,
                    "missing module metadata",
                    "a rooted module or direct import has no static module declaration",
                )
                .with_subject("requested module"),
            )),
            [descriptor] => Ok(*descriptor),
            [first, rest @ ..] => {
                let primary = Diagnostic::new(
                    FURNACE008,
                    "ambiguous module metadata",
                    "a module type has more than one static declaration",
                )
                .with_subject(first.type_name())
                .with_location(first.location());
                let related = rest.iter().map(|descriptor| {
                    Diagnostic::new(
                        FURNACE008,
                        "conflicting module declaration",
                        "this declaration describes the same module type",
                    )
                    .with_subject(descriptor.type_name())
                    .with_location(descriptor.location())
                });

                Err(Error::from_diagnostics(primary, related))
            }
        }
    }

    /// Ensures the root module has a node before its registration is traversed.
    fn push_node(&mut self, descriptor: &CauldronDescriptor) -> Result<usize> {
        self.ensure_node(descriptor)
    }

    /// Returns an existing node or appends one after validating namespace metadata.
    fn ensure_node(&mut self, descriptor: &CauldronDescriptor) -> Result<usize> {
        if let Some(index) = self.node_index(descriptor.type_id()) {
            return Ok(index);
        }

        let Some(namespace) = descriptor.namespace() else {
            return Err(Error::new(
                Diagnostic::new(
                    FURNACE008,
                    "missing module namespace",
                    "rooted module graphs require namespace metadata",
                )
                .with_subject(descriptor.type_name())
                .with_location(descriptor.location()),
            ));
        };
        let index = self.cauldrons.len();
        self.node_ids.insert(descriptor.type_id(), index);
        self.cauldrons.push(CauldronNode {
            type_id: descriptor.type_id(),
            type_name: descriptor.type_name(),
            namespace,
            location: descriptor.location(),
            definition: None,
        });

        Ok(index)
    }

    /// Records a directed edge from an importing module to its direct import.
    fn push_edge(&mut self, importer: usize, imported: usize) {
        self.imports.push(CauldronImportEdge { importer, imported });
    }

    /// Finds a reachable node by its concrete module type.
    fn node_index(&self, type_id: TypeId) -> Option<usize> {
        self.node_ids.get(&type_id).copied()
    }

    /// Builds an ordered diagnostic chain for a cycle in module imports.
    fn module_cycle_error(&self, cycle: &[TypeId], repeated: TypeId) -> Error {
        let mut chain = cycle
            .iter()
            .filter_map(|type_id| self.descriptor_for_stack(*type_id))
            .collect::<Vec<_>>();

        if let Some(repeated) = self.descriptor_for_stack(repeated) {
            chain.push(repeated);
        }

        let subject = chain
            .iter()
            .map(|descriptor| descriptor.type_name())
            .collect::<Vec<_>>()
            .join(" -> ");
        let primary_descriptor = chain
            .first()
            .expect("cycle members were already resolved during traversal");
        let primary = Diagnostic::new(
            FURNACE008,
            "module import cycle",
            "reachable module imports form a cycle",
        )
        .with_subject(subject.clone())
        .with_location(primary_descriptor.location());
        let related = chain
            .iter()
            .skip(1)
            .take(chain.len().saturating_sub(2))
            .map(|descriptor| {
                Diagnostic::new(
                    FURNACE008,
                    "module in import cycle",
                    "this module participates in the import cycle",
                )
                .with_subject(subject.clone())
                .with_location(descriptor.location())
            });

        Error::from_diagnostics(primary, related)
    }

    /// Finds the declaration metadata for one module currently on the DFS stack.
    fn descriptor_for_stack(&self, type_id: TypeId) -> Option<&'static CauldronDescriptor> {
        self.by_type.get(&type_id)?.first().copied()
    }
}

/// Creates a uniform rooted-registration diagnostic with the offending declaration location.
fn registration_error(
    title: &'static str,
    subject: &'static str,
    location: SourceLocation,
    requester: &'static str,
) -> Error {
    Error::new(Diagnostic::new(FURNACE008, title,
        format!("cauldron `{requester}` must register unique members and imports, and export only its local providers"))
        .with_subject(subject).with_location(location))
}
