# Password Hasher Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task by task.

**Goal:** Provide a replaceable, clonable password hasher using the existing Logger pattern.

**Architecture:** `PasswordHasherService: Send + Sync` defines synchronous `hash_password(&str)` and `validate_password(&str, &str)` methods returning the existing Argon2 error type. `PasswordHasher` shares an `Arc<dyn PasswordHasherService>`. `DefaultPassowrdHasher` wraps one configured Argon2 instance. A global `PasswordHasherCauldron` supplies the default and supports application builder overrides.

**Tech Stack:** Rust, existing Argon2 0.5, Furnace injector metadata.

**Spec:** User approved proceeding with the logger pattern in this conversation.

## Global Constraints

- Preserve free helper signatures and validation semantics, fresh salts, and synchronous execution.
- Keep integration behind `password-hasher`, independent of HTTP.
- Build on the existing uncommitted draft; leave unrelated editor files alone.
- Export new contracts through common, facade, and prelude.

## Review Focus

- Custom backend errors must propagate unchanged.
- Clones must share the backend.
- Configured Argon2 parameters must govern hashes; stored parameters govern verification.
- Default injection must reach dependent modules and accept supplied overrides.
- Existing malformed hash, mismatch, unsupported algorithm, and UTF-8 semantics must remain covered.

### Task 1: Instance abstraction and default implementation

**Files:** `crates/furnace-rs-common/src/password_hasher/{mod.rs,default.rs,password_hasher_service.rs}`, `crates/furnace-rs-common/src/lib.rs`, `crates/furnace-rs-common/tests/password_hasher.rs`.

- [x] Write integration tests for default and configured instances, backend replacement, shared clones, and error propagation.
- [x] Run `cargo test -p furnace-rs-common --no-default-features --features password-hasher --test password_hasher` and observe missing API failures.
- [x] Implement service, facade, configured Argon2 backend, provider metadata and global module. Delegate old helpers to the default backend.
- [x] Verify integration and existing module tests.

### Task 2: Public exports and documentation

**Files:** `crates/furnace-rs/src/lib.rs`, `crates/furnace-rs/tests/password_hasher.rs`, `docs/password-hasher.md`.

- [x] Extend facade tests to cover root, common, module, and prelude exports.
- [x] Add facade exports and document configuration, custom implementations, and injection.
- [x] Run facade tests, common tests and doctests with only `password-hasher`; check formatting of changed Rust files.

## Execution record

- Proceeding inline in the existing checkout to incorporate the user's draft. No branch or unrelated file changes requested. The user subsequently requested a commit with TDD verification.

- Verification: common 7 unit tests and 5 integration tests passed; facade 2 integration tests passed. Common rustdoc examples: 2 passed. Formatting and diff whitespace checks passed.
- Independent code review: no actionable findings.
- Facade doctests with only password-hasher: 2 passed, 2 failed in unchanged crate-level HTTP/runtime examples because their required features are disabled; unrelated documentation left intact.

### Commit verification

- RED: temporarily replaced the shared `Arc` clone with construction of a new default backend. `clones_passed_to_different_functions_share_backend_state_and_lifetime` failed at its first backend-state assertion (`secret:1`), as expected. Restored the original implementation in a `finally` block.
- GREEN: 7 common unit tests, 6 common integration tests, 2 facade integration tests, and 3 common doctests passed with only `password-hasher` enabled.
- Formatting and staged whitespace checks passed. No production changes were needed for the latest regression test.
- Broad `cargo test` run was interrupted during lengthy consumer-compilation fixtures after the targeted checks passed; no failures had been reported. The full workspace suite is not claimed as passing.
