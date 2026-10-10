# Unix scaffold directory privacy

Baseline: `bc16a4fc20a7a03ff13cda98de93121fbd2a8541` (1.0.2 source).
Affected implementation: `furnace-rs-cli/src/scaffold/publish.rs`.

## Reproduction and prerequisites

The CLI described its staging directory as private but created it using
`std::fs::create_dir`. With umask `000`, its mode was `0777`; with `002`, it was
`0775`. Atomic publication preserved those permissions on the final project.
In a shared, traversable invocation directory, another user or writable group
member could modify files or add an extra `build.rs` before publication. Cargo
later treats `build.rs` as executable build code if the owner builds the project.

The regression launches isolated child test processes with umasks `000`, `002`,
`022`, and `077`, so the parent test runner's umask never changes. It checks the
staging root before the first file is written, the published root, and the six
generated file contents. A completion sentinel prevents an empty child test
selection from appearing successful.

```sh
cargo test --locked -p furnace-rs-cli --lib scaffold_directories_are_private_under_each_umask -- --nocapture
```

Before the patch, the regression failed at `staging mode was 777`. An additional
production-source probe observed `0775` under `002` and demonstrated that an
extra file inserted by the owner-level preparation hook survived publication.
That hook demonstrates publication of unverified extras; it is not a separate
UID attack. No live cross-user code-execution exploit was executed. The
cross-user write opportunity follows from the measured POSIX directory modes
and requires a traversable shared parent and a permissive umask.

## Patch method and compatibility

Create Unix staging directories with `DirBuilderExt::mode(0o700)` on mkdir itself,
before generating any contents. This avoids a create-then-chmod access window.
Non-Unix behavior is preserved. Rename retains the private root permissions on
the generated project; owners can explicitly grant sharing permissions later.
Shared-parent rename/removal attacks depend on the parent's permissions and are
not claimed to be solved by this patch.

The four-umask regression and the complete publication unit suite pass after the
fix, and the scaffold CLI integration suite preserves ordinary generation.

Evidence: [baseline](evidence/2026-10-10-staging-red.txt),
[patched](evidence/2026-10-10-staging-green.txt).
