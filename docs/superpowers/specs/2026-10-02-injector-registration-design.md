# Injector construction and explicit provider bindings

Date: 2026-10-02
Status: Approved, implemented, and verified on stable Rust and Rust 1.94; see the Injector registration verification report.
Branch: `refactor/furnance-injector`

## Intent and agreed API

Replace `#[element]` factory declarations with an explicit `Injector` construction contract. A cauldron registers ordinary services without a factory argument and chooses an implementer when binding a trait output. Registration must remain free of construction and I/O, and preserve dependency validation, cauldron visibility, asynchronous construction, and lifecycle cleanup.

The user approved this registration design:

```rust
self
    .provide::<UserService>()
    .provide_with::<Arc<dyn UserRepository>, PostgresRepository>()
    .controller::<UserController>()
```

Rust does not support optional method arguments or overloads by argument count. These two methods express the intended distinction. Neither method accepts a constructed service instance. The second type parameter selects a constructor at the type level.

`#[burner]` and `#[storage]` remain convenient managed-service declarations and generate `Injector` implementations. Plain structs may implement `Injector` manually. The controller and seal APIs, including public controllers and `#[seal(skip)]`, retain their current behavior.

## Construction contract

The public contract is:

```rust
pub trait Injector<T = Self>: Sized + Send + Sync + 'static
where
    T: Send + Sync + 'static,
{
    type Dependencies: InjectionDependencies;

    fn inject(
        dependencies: Self::Dependencies,
    ) -> impl Future<Output = Result<T>> + Send;

    fn lifecycle(value: T) -> LifecycleResource<T> {
        LifecycleResource::new(value)
    }
}
```

`inject` is an associated constructor, not an instance method. Implementations can use `async fn inject(...) -> Result<T>`. There is no receiver because obtaining a receiver would require constructing the service before its dependencies are ready. Manual callers may also invoke and await the constructor with an explicit dependency tuple.

`InjectionDependencies` describes and resolves `()` and tuples with one through sixteen elements. Every dependency is `Clone + Send + Sync + 'static`; tuple position preserves declaration order. One dependency is written `(Dependency,)`. The runtime resolves and clones existing provider handles, matching the current managed/provider behavior. Dependency metadata uses each concrete Rust type's identity and runtime name.

Dependencies are explicit inputs rather than unrestricted registry access. This preserves pre-construction missing-dependency, cycle, and visibility checks. Configuration is a declared `Config` dependency when needed. Construction errors use the existing core `Result` and diagnostic propagation; the future must be `Send`.

Example, assuming `UserRepository: Send + Sync`:

```rust
impl Injector<Arc<dyn UserRepository>> for PostgresRepository {
    type Dependencies = (DatabaseConnection,);

    async fn inject((database,): Self::Dependencies)
        -> Result<Arc<dyn UserRepository>>
    {
        Ok(Arc::new(Self { database }))
    }
}
```

The implementation explicitly constructs and coerces its result to the declared output. The framework does not guess trait coercions or add another public `Arc` layer. The registry's internal erased wrapper retains its existing behavior.

## Lifecycle resources

An ordinary injector returns its service value. The runtime then calls `I::lifecycle(value)` exactly once and retains both its native output and lifecycle hooks. The default contributes no hooks. An injector requiring hooks overrides this method and uses the existing `LifecycleResource::with_application_hook` or `with_infrastructure_hook` builders.

This replaces both ordinary and lifecycle forms of `#[element]`. Asynchronous resource acquisition belongs in `inject`; lifecycle attachment is synchronous and cannot fail. Existing startup ordering, rollback, reverse shutdown, and provider-contribution cleanup remain in force. Database connections remain native `DatabaseConnection` outputs and contribute their current infrastructure hook.

For native outputs owned by another crate, use a local constructor type implementing `Injector<NativeOutput>` and register `.provide_with::<NativeOutput, LocalConstructor>()`. This is necessary for Rust's orphan rules: persistence cannot implement a core-owned trait directly on SeaORM's type. Therefore `provide_with` primarily serves trait bindings but also supports native third-party outputs; it cannot be restricted to `Arc<dyn Trait>` alone.

## Registration and metadata

`CauldronRegistration::provide<T>()` selects `T: Injector<T>`. `provide_with<T, I>()` selects `I: Injector<T>` and records `T` as the owned output. The cauldron macro exposes both entry methods. Exports and consumers continue to name the output, never the constructor type.

Manual injectors require no inventory macro. Their typed registrations contribute constructor descriptors to rooted analysis. Descriptor metadata is available without calling `inject` or `lifecycle`; repeated analysis must not leak descriptor allocations. Reuse the existing static-descriptor model with references to generic associated constant descriptors where possible. Verify this mechanism on Rust 1.94 before integrating it.

Managed service/storage/controller descriptors remain discoverable in the linked catalog. Managed generated constructors delegate to their generated injector contract, preserving managed handle cloning, fields, role metadata, and controller callbacks. Selected explicit constructor descriptors are authoritative for their output; an unselected linked implementation must not create a false ambiguity. Two explicit registrations of the same output still produce duplicate/ownership diagnostics, even when their implementers differ.

Rooted builds collect manual descriptors from reachable cauldrons before provider selection, graph validation, HTTP preflight, and construction planning. Manual injectors from unrelated cauldrons must not enter the selected dependency graph or create unrelated errors. Whole-catalog and unrooted focused builds discover macro-generated descriptors only; manual registrations require selecting their owning cauldron. Existing unrooted APIs must document this boundary. Focused managed construction continues to work without a cauldron root.

Cauldron import/export/global rules remain output-based. Registering a trait implementation does not implicitly publish its concrete implementer or grant access to its dependencies. The existing single reachable owner per output remains in force; this change does not introduce separate instances of the same output in different cauldrons.

## Removal and migration

Remove the public `element` proc macro, facade/core reexports, and generated free-function factory metadata path. There is no deprecated alias. Migrate framework integrations, examples, tests, scaffold guidance, and active documentation to injectors. Ordinary helper functions may remain when useful, but registration must use an injector rather than depend on a factory attribute.

Native integration providers may keep deliberately authored low-level descriptors when their auto-configuration integration requires catalog discovery. These constructors must delegate to the injector contract and retain existing focused-selection requirements. Do not replace official opt-in integration behavior with global construction of every registered manual injector.

Historical specifications and reports remain historical records. Update the active migration guide, Rust API documentation, architecture guide, and Unreleased changelog. Do not rename packages, bump versions, publish, or push. Commit completed implementation tasks on the assigned branch.

## Validation and acceptance

- Compile and run plain injectors, zero/one/multiple dependencies, async errors, and managed auto-generated injectors on stable and Rust 1.94.
- Compile failures cover incompatible output bindings, unsupported dependency shapes, non-clone dependencies, non-Send construction futures, and references to the removed macro.
- Prove that registration, inspection, and failing preflight perform no construction or lifecycle attachment.
- Verify missing dependencies, cycles, cauldron visibility, duplicate ownership, explicit binding selection, imports/exports/global outputs, and isolation from unrelated cauldrons.
- Resolve trait outputs and concrete outputs with the existing registry API; verify construction happens once and clones share managed state.
- Verify lifecycle startup, rollback, reverse shutdown, and cleanup after errors for manual and integration injectors.
- Preserve direct controller routing, optional seals, endpoint skips, inspector output, CLI scaffolds, feature combinations, and external crate aliases.
- Migrate and build all standalone examples and verify package contents after staging new tracked files.
- Run formatting, strict Clippy, workspace tests, doctests, strict documentation, and MSRV validation after migration.
- Run PostgreSQL persistence and database recovery checks against a temporary local cluster or Docker, then stop and remove only resources created for verification.

This specification intentionally excludes changes to HTTP routing, authentication policies, provider lifetimes, the single-owner scope model, package branding, and historical benchmark artifacts.
