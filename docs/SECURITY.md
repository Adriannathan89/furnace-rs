# Security policy

The current released version is furnace-rs **1.0.1**, released on **2026-10-06**.
Use 1.0.1 for the latest released security and availability fixes.

Version 1.0.1 adds ten-second initial-request/header and idle-body deadlines,
rejects zero database maintenance durations, and validates inventory output
types during construction. See the [release notes](releases/1.0.1.md),
[HTTP runtime report](../benchmark/HTTP_RUNTIME_SECURITY.md), and
[database/inventory report](../benchmark/DATABASE_INVENTORY_SECURITY.md).
The [1.0.0 audit](SECURITY_AUDIT.md) retains evidence for earlier fixes.

This branch also includes the Unreleased
[validated-extractor error-redaction fix](security/2026-10-09-extractor-error-redaction.md).
That fix is present in the checkout; it is not part of the published 1.0.1 release.

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
integration. The current development branch and released 1.0.x line are the
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
