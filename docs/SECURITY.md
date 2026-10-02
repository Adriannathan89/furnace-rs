# Security policy

This repository is preparing furnace-rs **1.0.0**. The version has been
published at 2026-10-3. Security fixes described in the
[audit](SECURITY_AUDIT.md) are included in the prepared source.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting option on the repository's
[Security page](https://github.com/Adriannathan89/furnace-rs/security) when it is
available. If private reporting is unavailable, open an issue asking for a
private reporting channel without posting exploit details or credentials.
No dedicated security email address or response-time guarantee is established
by this document.

Include the affected package/version or commit, enabled features, platform,
the trust boundary involved, minimal reproduction, expected/actual behavior,
and relevant sanitized logs. Use synthetic credentials and disposable databases.
Distinguish a confirmed exploit from a source-level hypothesis.

## Scope and maintenance

The security review covers the nine `furnace-rs` workspace crates, their normal
dependency graph, CLI control transports, and documented authentication/database
integration. The current development branch and forthcoming 1.0.x line are the
focus of this policy. This does not promise backports to every historical MADS
or furnace-rs 0.x release.

Applications own their authorization policy, credential validation, password
hashing, CSRF defenses, token revocation/rotation, database query construction,
schema migrations, deployment key files, TLS, and request/resource limits beyond
the framework's documented defaults. Explicit `source()` access can reveal
native errors; authors must keep their own diagnostic messages safe.

## Security verification

Stable and beta publishing require a security job that checks the workspace
lockfile against RustSec advisories, verifies benchmark report integrity, and
runs the core/database and Unix CLI isolation contracts. CI runs the same checks.
The advisory command excludes registry yank status; it does not establish the
absence of undisclosed vulnerabilities.

Reproduction steps, original/updated benchmark results, real PostgreSQL
validation, and untested areas are indexed in [SECURITY_AUDIT.md](SECURITY_AUDIT.md).
Historical evidence remains tied to its original commit and source hash.
