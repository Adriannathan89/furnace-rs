# 2.0.0-beta.2 review findings

Reviewed on 2026-10-15 at commit `bd8ade6aa4759ad5072dd82a63b7c2006c31affc`, branch `test/beta-v2-validation-test`.

**Status: all 14 findings patched and validated.** The original findings and reproduction notes below describe the reviewed baseline. The requested column `serenity` means **severity**. High means a demonstrated security boundary failure; Medium means a functional failure or conditional dependency security exposure; Low means a narrower compatibility or response behavior problem. These are review priorities, not CVSS scores.

## Remediation

| Finding | Implemented correction | Regression evidence |
| --- | --- | --- |
| F01 | Conditional method gates also control policy metadata, including nested `cfg_attr`; conditional helper declarations get an explicit diagnostic. | [Before/after comparison](../benchmark/GRPC_POLICY_SECURITY.md), [machine-readable evidence](../benchmark/results/2026-10-15-grpc-policy-before-after.json), and [conditional attributes](../crates/furnace-rs-grpc/tests/grpc_conditional_attributes.rs). |
| F02 | Three Rustls locks updated to 0.23.46; CI audits every committed lockfile. | [Audit runner](../benchmark/tool/audit_lockfiles.py); all 11 committed locks pass the cached RustSec scan. |
| F03 | Lifecycle progress and failures survive interruption; shutdown resumes cleanup of completed starts and retries pending stops. | [Cancellation regressions](../crates/furnace-rs-core/tests/lifecycle.rs). Hooks retain documented responsibility for partial startup effects and retry-safe stops. |
| F04 | Unsupported conditional controller fields are rejected before invalid dependency code is emitted. | [Compile-fail fixture](../crates/furnace-rs/tests/ui/fail/controller_conditional_fields.rs), plus macro tests. |
| F05 | Newtypes and transparent wrappers omit artificial path segments while preserving enum tag/content prefixes. | [Real JSON validation tests](../crates/furnace-rs-common/tests/input_validation.rs). |
| F06 | Cargo builds retain the invocation directory and its Cargo configuration scope. | [Cargo execution fixtures](../crates/furnace-rs-cli/tests/cargo_execution.rs). |
| F07 | Application launch modes share dynamic library path setup using build and toolchain information. | [Cargo execution fixtures](../crates/furnace-rs-cli/tests/cargo_execution.rs), including real dynamic linking. |
| F08 | Development watching includes protobuf and declared build-script inputs while filtering runtime output. | [Build-input/restart fixtures](../crates/furnace-rs-cli/tests/cargo_execution.rs). |
| F09 | Manifest and lockfile changes refresh metadata, target selection, exclusions, and watch registrations. | [Dependency add/edit/remove fixtures](../crates/furnace-rs-cli/tests/cargo_execution.rs) and watcher tests. |
| F10 | Unsupported empty and literal dotted TOML key segments are rejected with value-free diagnostics. | [Configuration regressions](../crates/furnace-rs-core/tests/config.rs). |
| F11 | Later incompatible shapes remove obsolete values, descendants, and provenance; compatible tables merge by field. | [Source precedence regressions](../crates/furnace-rs-core/tests/config.rs). |
| F12 | Generated gRPC helpers live in an anonymous constant scope. | [Qualified service compile/runtime metadata regression](../crates/furnace-rs-grpc/tests/grpc_symbol_scope.rs). |
| F13 | Response additions/removals use the request parser's original cookie-name policy before headers are emitted. | [Cookie response regressions](../crates/furnace-rs-common/tests/cookie_response.rs). |
| F14 | Timeout normalization retains middleware headers/extensions and replaces obsolete body metadata. | [HTTP/1.1 and HTTP/2 stalled-upload regressions](../crates/furnace-rs-common/src/server.rs). |

## Current validation and commits

The workspace was validated in two test groups, both with all features and local
socket/signal permissions: the non-CLI workspace reported **778 passes**, and the
CLI suite reported **174 passes**, including two nested fixture executions. All
**7 ignored tests require PostgreSQL**. After the final small watcher path
normalization change, all **7 watcher tests** passed again. Both groups passed
all-target Clippy with warnings denied; workspace formatting and whitespace
checks passed. The security evidence Python suite passed **21 tests**. Every
committed lockfile passed the cached RustSec scan described below.

The F01 fixture failed its security assertion on the archived baseline and passed
on the patch: anonymous guarded calls rejected **0/200 → 200/200**, authenticated
controls **200/200 → 200/200**, public controls **200/200 → 200/200**. Source,
fixture, and runner fingerprints match the stored [comparison evidence](../benchmark/results/2026-10-15-grpc-policy-before-after.json).
See the [validation record](../benchmark/results/2026-10-15-beta-remediation-validation.json)
for commands and scope. Linux was exercised; Windows/macOS runtime and live
database testing were not performed. Lifecycle hooks retain the documented
partial-start cleanup and retry-safe stop obligations.

| Commit | Batch | Findings |
| --- | --- | --- |
| `ce2aa46` | Core lifecycle and configuration | F03, F10, F11 |
| `2520bad` | HTTP, validation, cookies, controller macros | F04, F05, F13, F14 |
| `6245c95` | Rustls lockfiles and CI/release advisory gates | F02 |
| `b14130f` | gRPC policy, helper isolation, before/after evidence | F01, F12 |
| `286c1c7` | Cargo execution and development watcher lifecycle | F06–F09 |

The resolution report and validation record are committed in a final
documentation batch. Changes remain on the assigned branch.

## Findings

| name | serenity | type | affected | recomendation-solution |
| --- | --- | --- | --- | --- |
| F01 — Disabled gRPC method override removes active authentication | **High** | Security: authorization bypass | [grpc_service.rs:194](../crates/furnace-rs-grpc-macros/src/grpc_service.rs#L194), [scope.rs:234](../crates/furnace-rs-grpc/src/grpc/scope.rs#L234). A method disabled with `#[cfg(any())]` still contributes its `public` override. An active conditional implementation of the same RPC consequently loses the service guard. **An anonymous in-process RPC returned successfully in the executed probe.** | Apply conditional attributes consistently to methods and all generated policy metadata. Handle `cfg_attr` as well as `cfg`, or reject unsupported conditional declarations explicitly. Add regression coverage for disabled public/guard overrides and active replacements; require active policy to govern admission. |
| F02 — Three committed lockfiles retain vulnerable Rustls | **Medium, conditional** | Security: dependency versions; verification coverage | [posts-crud lock:1351](../example/posts-crud/Cargo.lock#L1351), [post-crud-with-service lock:1351](../example/post-crud-with-service/Cargo.lock#L1351), [Axum benchmark lock:999](../benchmark/targets/axum/Cargo.lock#L999) pin Rustls `0.23.45`. Each matches four published RustSec advisories. The workspace lockfile uses the patched version; CI's advisory command checks only that workspace lockfile. | Update the three lockfiles to Rustls `>=0.23.46` and audit every committed lockfile in CI. Check feature and call-path applicability before asserting exploitability: the advisory matches alone do not demonstrate exploitation in these applications. See advisory details below. |
| F03 — Cancelling lifecycle operations strands cleanup | **Medium** | Design: cancellation and resource lifecycle | [lifecycle.rs:184](../crates/furnace-rs-core/src/lifecycle.rs#L184), [lifecycle.rs:227](../crates/furnace-rs-core/src/lifecycle.rs#L227). Dropping a pending startup future loses its local list of successfully started hooks and leaves state `Starting`; shutdown then rejects with `FURNACE010`. Shutdown cancellation similarly leaves `Stopping`. | Store lifecycle progress in the manager and define a resumable cleanup path for interrupted operations. Ensure cancellation cannot make already started resources unreachable to shutdown. Document hook cancellation obligations and cover interrupted start, rollback, and stop. |
| F04 — Conditional controller dependencies generate invalid Rust | **Medium** | Correctness: procedural macro | [controller/managed.rs:99](../crates/furnace-rs-common-macros/src/controller/managed.rs#L99). Field conditional attributes reach the inner struct declaration but not dependency types, metadata, destructuring, or initialization. A disabled injected field produces `E0425` and `E0560`. This remains from the October 10 audit. | Design consistent conditional dependency generation across all artifacts. Until supported, reject conditional controller fields with a clear macro error instead of emitting invalid code. |
| F05 — Validation paths do not match newtype and transparent JSON | **Medium** | API mismatch: validation error locations | [input.rs:128](../crates/furnace-rs-common-macros/src/input.rs#L128), [serde_path.rs:8](../crates/furnace-rs-common-macros/src/input/serde_path.rs#L8). Newtype fields receive an artificial index; transparent named wrappers receive an artificial field name. Errors therefore identify JSON locations that do not exist. | Model Serde newtype and transparent representations when constructing paths. Add tests that deserialize actual JSON before validation, including enum tagging variants, and correct the existing one-field variant expectation. |
| F06 — Cargo builds lose member-local configuration | **Medium** | Correctness: CLI project execution | [cargo.rs:35](../crates/furnace-rs-cli/src/cargo.rs#L35). Project discovery starts at the invocation directory, but compilation runs from the workspace root. Member `.cargo/config.toml` settings can disappear. A project using member-local `[env]` compiled under `cargo run` and failed under `furnace run`. | Retain the original Cargo invocation directory and use it for build commands in run, dev, and inspection. Verify member-local environment, linker, target, and rustflags settings. |
| F07 — Direct process launch omits Cargo library search paths | **Medium** | Correctness: CLI runtime environment | [process.rs:161](../crates/furnace-rs-cli/src/process.rs#L161), related dev/inspection launches. With `RUSTFLAGS='-C prefer-dynamic'`, the build succeeds but `furnace run` exits 127 because the loader cannot find Rust's shared standard library; `cargo run` succeeds. | Reproduce Cargo's platform-specific dynamic library execution environment for all direct launches, including toolchain and relevant build/dependency directories. Preserve existing user search paths and test each supported platform. |
| F08 — Development mode ignores protobuf inputs | **Medium** | Correctness: gRPC development workflow | [watch.rs:164](../crates/furnace-rs-cli/src/watch.rs#L164), [gRPC example build.rs:2](../example/grpc-transport/build.rs#L2). Edits to `.proto` files are excluded even when the build script declares them through `cargo:rerun-if-changed`. Running services retain the old schema until another recognized file changes. | Include protobuf inputs in reachable packages and account for other build-script inputs. Use Cargo build-script metadata where practical; ensure an edited protobuf causes one rebuild/restart. |
| F09 — Development watcher freezes the initial dependency graph | **Medium** | Design: CLI watch lifecycle | [dev.rs:56](../crates/furnace-rs-cli/src/dev.rs#L56), [dev.rs:134](../crates/furnace-rs-cli/src/dev.rs#L134). Adding a path dependency triggers a manifest rebuild, but watcher roots and package exclusions remain based on the original metadata. Later edits in the new dependency do not rebuild the application. | Reload project metadata and replace watcher registrations/exclusions after relevant manifest or dependency-resolution changes. Re-resolve the selected target and report invalidated selections clearly. |
| F10 — Distinct TOML keys silently collapse to one setting | **Medium** | Correctness: configuration identity | [config.rs:416](../crates/furnace-rs-core/src/config.rs#L416). The valid TOML keys `"app.port" = 9000` and `[app] port = 3000` both flatten to `app.port`. The document loads successfully with one entry and silently selects `9000` in the probe. | Reject unsupported dotted/empty literal segments and flattened-key collisions, or preserve structured key segments internally. Return a value-free configuration diagnostic instead of silently replacing a distinct key. |
| F11 — Configuration merges retain incompatible table/scalar shapes | **Medium** | Design: configuration source precedence | [config.rs:782](../crates/furnace-rs-core/src/config.rs#L782), [parse.rs:8](../crates/furnace-rs-core/src/configuration/parse.rs#L8). A later `[app]` table leaves an earlier scalar `app` in storage. Nested parsing then rejects the obsolete scalar. Conversely, a later scalar leaves earlier table metadata and children present. | Define precedence for table/scalar/array changes and enforce it across values, tables, descendants, and source provenance. Preserve field-wise merges between compatible tables. Test both directions of shape replacement. |
| F12 — gRPC helper names collide for distinct qualified types | **Medium** | Correctness: procedural macro namespace | [grpc_service.rs:145](../crates/furnace-rs-grpc-macros/src/grpc_service.rs#L145). Helpers use only the final type identifier. Implementations for `a::Implementation` and `b::Implementation` written in one module generate duplicate registrar and descriptor names (`E0428`), even if applications would select them separately. | Place generated helpers inside an anonymous constant or another isolated scope, or generate identifiers unique to the complete declaration. Add a consumer compile test with equal final type names from different modules. |
| F13 — Emitted cookie names fail the framework's own request parser | **Low** | API mismatch: cookie round trip | [cookie.rs:315](../crates/furnace-rs-common/src/cookie.rs#L315), [cookie.rs:261](../crates/furnace-rs-common/src/cookie.rs#L261). `Cookie::new("name with space", "v")` emits `name%20with%20space=v`, but returning that cookie produces `MalformedRequest`. It can break cookie extraction/authentication even when unrelated to the selected token. Requires an application choosing such a name; remains from the October 10 audit. | Validate response cookie names against the request name policy before emission, or define a consistent encoded-name round trip. Align response/request tests. |
| F14 — Body timeout normalization drops middleware headers | **Low** | Correctness: HTTP response composition | [server.rs:396](../crates/furnace-rs-common/src/server.rs#L396). The runtime awaits the router response, then replaces it with a fresh 408 when body reading expired. CORS, `Vary`, request correlation, and security headers supplied by application middleware are lost. | Normalize timeout responses inside the application middleware boundary, or preserve appropriate headers while replacing status/body and removing obsolete content headers. Verify CORS and correlation behavior on stalled uploads. |

## Evidence and reproduction notes

### F01 — gRPC policy mismatch

The temporary consumer used the repository's generated `Hidden` service and descriptor. Its service declared `guard = Protected`; the implementation contained these mutually exclusive methods:

```rust
#[cfg(any())]
#[grpc_method(name = "Secret", public)]
async fn secret(/* ordinary generated RPC signature */) { /* disabled */ }

#[cfg(not(any()))]
async fn secret(/* same signature */) { /* active implementation */ }
```

Two executed assertions established that the declared service guard was present while the selected method guard was absent, and that the generated client could call the active handler without credential metadata. The active handler returned the synthetic value `42` through `build_grpc_routes(...).prepare()`. The probe required no network listener. Inspection of [router.rs:100](../crates/furnace-rs-grpc/src/grpc/router.rs#L100) and [passport.rs:98](../crates/furnace-rs-grpc/src/grpc/passport.rs#L98) confirms that this missing method guard becomes a public dispatch binding.

This requires an application with conditional implementations and a disabled public override. It does not demonstrate a bypass for every guarded gRPC service. The same missing condition can also leave disabled guard type references in generated code and prevent compilation.

### F02 — Advisory matches and their limits

Ran `cargo audit --no-fetch --no-yanked --file <lockfile>` over all 45 local lockfiles outside build directories, including **all 11 committed lockfiles**. Only the three committed lockfiles in F02 returned vulnerability matches. The workspace scan covered 374 dependencies and returned no matches. The cached advisory database was commit `caee11f4`, dated 2026-10-14; the four advisory pages were also checked online on the review date.

All four advise updating to Rustls `>=0.23.46`:

- [RUSTSEC-2026-0346](https://rustsec.org/advisories/RUSTSEC-2026-0346.html): TLS session secrets can appear in custom session-store debug logging; ordinary built-in cache debug output is unaffected.
- [RUSTSEC-2026-0347](https://rustsec.org/advisories/RUSTSEC-2026-0347.html): buffered application data can reach an authenticated outer ECH server after ECH rejection; requires specific ECH and buffering behavior.
- [RUSTSEC-2026-0348](https://rustsec.org/advisories/RUSTSEC-2026-0348.html): certificate decompression can allocate excessive memory; requires the nondefault `brotli` feature and an applicable TLS peer path.
- [RUSTSEC-2026-0349](https://rustsec.org/advisories/RUSTSEC-2026-0349.html): unauthenticated key-share prediction can affect later post-quantum negotiation, depending on server policy and session storage. The advisory does not claim an authentication bypass.

The scan confirms affected versions in lockfiles, not that all four conditions are reachable in the checked-in examples. Registry yanks were excluded. [CI](../.github/workflows/ci.yml) currently audits only `Cargo.lock` at the workspace root.

### F03, F10, F11 — Core probes

Executed a standalone Rust consumer linked against the current core library:

```text
startup: first hook succeeds, second hook remains pending, startup future dropped
observed state: Starting
observed stop calls: 0
subsequent shutdown: FURNACE010, cannot shutdown while Starting

TOML: "app.port" = 9000; [app] port = 3000
observed app.port: 9000
observed entry count: 1

source 1: app = "old scalar"
source 2: [app] port = 3000
observed: scalar app still present, table app present, app.port = 3000
```

The lifecycle failure was executed for startup cancellation; the equivalent shutdown cancellation follows from the same state-before-await pattern and was reviewed in source. No remote resource leak was measured. Configuration examples contain synthetic values only.

### F04, F05, F13 — HTTP/macro probes

An authored controller with `#[cfg(any())] #[inject] disabled: MissingDependency` failed compilation with `E0425` and `E0560` against the current source. This rechecks the earlier audit using beta injection syntax.

Deserialized real JSON before calling `Input::validate()`:

| Rust representation / JSON | Observed issue path | Expected JSON location |
| --- | --- | --- |
| `Newtype(String)` / `""` | `[0]` | root |
| `#[serde(transparent)] struct T { value: String }` / `""` | `["value"]` | root |
| `enum E { Name(String) }` / `{"Name":""}` | `["Name", 0]` | `["Name"]` |

Each string had a `nonempty` validator. The existing [validation test:135](../crates/furnace-rs-common/tests/input_validation.rs#L135) expects the extra index for a one-field variant, so passing that test does not establish JSON-path correctness.

Cookie execution emitted `Set-Cookie: name%20with%20space=v`; parsing `Cookie: name%20with%20space=v` returned `MalformedRequest`. No browser behavior was tested.

### F06–F09, F12, F14 — Execution and source evidence

- **F06:** In a temporary workspace member, `.cargo/config.toml` supplied an environment variable used by `env!`. `cargo run` printed `configured`; `furnace run` failed compilation because the variable was absent.
- **F07:** With `RUSTFLAGS='-C prefer-dynamic'`, `cargo run` printed `DYNAMIC_OK`; `furnace run` built successfully, then exited 127 with a missing `libstd-*.so` loader error. This was exercised on Linux; other platforms were reviewed in source.
- **F08:** A temporary app printed `SCHEMA=v1`. Changing its build-script-tracked `.proto` input caused no rebuild during the observation window. Touching `main.rs` then rebuilt and printed `SCHEMA=v2`. The watcher extension filter independently explains the omission.
- **F09:** After adding a local workspace dependency and completing the rebuild with `VALUE=v1`, editing that dependency to v2 left the build count unchanged and never printed `VALUE=v2`. Watcher registrations and exclusions are created once and retained through the loop.
- **F12:** A temporary consumer with qualified `a::Implementation` and `b::Implementation` service implementations failed with two `E0428` errors naming the generated registrar and descriptor.
- **F14:** Confirmed from response ordering in source. No new timed socket reproduction was executed; impact assumes middleware has supplied headers before the runtime replaces the response.

## Original scan scope and validation limits

Reviewed the 11 product crates: facade, core, core macros, common/HTTP/authentication, common macros, gRPC, gRPC macros, persistence, CLI, testing, and extra. Review covered dependency construction and module visibility, configuration and provenance, lifecycle ownership, HTTP/body handling, extraction and validation, cookies, JWT/Passport and guards, gRPC registration/policy/TLS/client/runtime paths, CLI subprocesses/watchers/inspection/scaffolding, feature manifests, release/CI workflows, and related examples, documentation, and tests. This is a broad source review with focused execution, not an exhaustive proof that every defect has been found.

- `cargo test --locked --offline -p furnace-rs-core --all-features` completed successfully: **192 tests passed**, including documentation tests; no failures or ignored tests.
- The workspace `cargo test --locked --offline --workspace --all-features` attempt compiled successfully and passed preceding suites, then stopped in `mixed_transports_share_provider_identity` when the sandbox denied a loopback bind (`Operation not permitted`). **A fully passing workspace run is not claimed.**
- `cargo test --locked --offline -p furnace-rs-grpc --all-features --lib` completed successfully: **12 unit tests passed**. The F01 in-process probe was independently rerun with two passing assertions, confirming the faulty behavior without sockets.
- Temporary consumers and probes lived under `/tmp`; existing build outputs were used. No regression tests or implementation changes were added to the repository.
- No live database audit, Windows/macOS runtime validation, browser exercise, fuzzing campaign, load test, publication, or deployment was performed. No JWT signature/algorithm bypass or additional concrete facade/testing/persistence defect was established by this review.

The original scan validation above is retained as historical review evidence;
current remediation checks and commits are recorded near the start of this file.
