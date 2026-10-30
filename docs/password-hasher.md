# Password hasher

The password hasher lives in
[`furnace-rs-common/src/password_hasher/mod.rs`](../crates/furnace-rs-common/src/password_hasher/mod.rs).
It uses Argon2 defaults, currently Argon2id version 19, to create and verify
password hashes.

## Feature selection and visibility

Enable `password-hasher` on `furnace-rs-common`, or on the `furnace-rs` facade,
which forwards the feature to common. This feature only adds the Argon2
dependency; it does not require HTTP. To select it alone in a workspace consumer:

```toml
[dependencies]
furnace-rs-common = { path = "../crates/furnace-rs-common", default-features = false, features = ["password-hasher"] }
```

Adjust the path for your consumer. Import the helpers from the crate root:

```rust
use furnace_rs_common::{hash_password, validate_password};

let hashed = hash_password("my password").expect("hash password");
assert!(validate_password("my password", &hashed).expect("parse stored hash"));
```

With the facade's `password-hasher` feature enabled, use
`furnace_rs::{hash_password, validate_password}` or `furnace_rs::prelude::*`.
The helpers are also available through `furnace_rs::common` and the
`password_hasher` module in either crate.

## Creating hashes

`hash_password(password: &str) -> Result<String, argon2::password_hash::Error>`
hashes the password's UTF-8 bytes using `Argon2::default()` and a new salt from
the operating system's random source.

The result is a PHC string with this structure:

```text
$argon2id$v=19$m=...,t=...,p=...$<salt>$<hash>
```

Store the entire returned string. It contains the algorithm, version, cost
parameters, salt, and hash needed for later verification. Repeated calls with
the same password produce independently salted hashes, so compare passwords
using validation instead of comparing hash strings.

Hashing and encoding errors propagate through `Err`. Salt generation can panic
if OS randomness is unavailable.

## Verifying passwords

`validate_password(password: &str, hashed: &str) -> Result<bool, argon2::password_hash::Error>`
parses the stored PHC string and verifies the supplied password with its encoded
settings.

| Result | Meaning |
| --- | --- |
| `Ok(true)` | The password matches. |
| `Ok(false)` | Verification failed, including a mismatch or unsupported algorithm. |
| `Err(error)` | The stored string could not be parsed as a PHC string. |

Every verification error after parsing is converted to `false`. A syntactically
valid PHC string with missing or unsupported settings therefore need not return
an error.

Both helpers accept empty and Unicode passwords. They operate on the supplied
bytes without trimming or Unicode normalization; password policy belongs to
the caller. Both functions perform synchronous work.

## Unit tests

Run the module tests without HTTP features:

```sh
cargo test -p furnace-rs-common --no-default-features --features password-hasher --lib password_hasher::tests
```

The tests cover the generated PHC format, matching and incorrect passwords,
independent salts, malformed strings, unsupported algorithms, empty passwords,
and exact UTF-8 matching.

## Reusable instances and custom backends

`PasswordHasher` is a clonable service facade, following the `Logger` pattern.
Clones share their backend through an `Arc`. The default backend is
`DefaultPassowrdHasher`, which owns one Argon2 instance:

```rust
use furnace_rs_common::PasswordHasher;

let hasher = PasswordHasher::default();
let hashed = hasher.hash_password("my password")?;
assert!(hasher.validate_password("my password", &hashed)?);
# Ok::<(), furnace_rs_common::PasswordHasherError>(())
```

To customize the Argon2 algorithm, version, or cost settings, pass a configured
instance. Add `argon2 = "0.5"` to your application's dependencies to construct it:

```rust
use argon2::{Algorithm, Argon2, Params, Version};
use furnace_rs_common::{DefaultPassowrdHasher, PasswordHasher};

let params = Params::new(19_456, 2, 1, None)?;
let backend = DefaultPassowrdHasher::new(Argon2::new(
    Algorithm::Argon2id,
    Version::V0x13,
    params,
));
let hasher = PasswordHasher::new(backend);
# Ok::<(), argon2::Error>(())
```

The backend reads cost settings from the Argon2 instance when hashing, and from
the stored PHC string when validating. Every hash still gets a fresh salt.

Applications can implement `PasswordHasherService: Send + Sync` and pass their
backend to `PasswordHasher::new`. Both methods are synchronous:

```rust
fn hash_password(&self, password: &str) -> PasswordHasherResult<String>;
fn validate_password(&self, password: &str, hashed: &str) -> PasswordHasherResult<bool>;
```

`PasswordHasherResult<T>` uses `PasswordHasherError`, a public alias for the
existing `argon2::password_hash::Error`. The facade forwards backend results
unchanged. Custom backends define their own validation failure behavior.

All these contracts are exported by `furnace_rs_common`, `furnace_rs`, and
`furnace_rs::prelude` with the `password-hasher` feature enabled.

## Dependency injection

Import the global `PasswordHasherCauldron` to inject the default hasher into
services, including services declared in other imported cauldrons:

```rust
use furnace_rs_common::{PasswordHasher, PasswordHasherCauldron};

#[furnace_rs_common::core::burner]
struct AuthService {
    #[inject]
    hasher: PasswordHasher,
}

#[furnace_rs_common::core::cauldron(
    imports = [PasswordHasherCauldron],
    providers = [AuthService],
)]
struct AppCauldron;
```

For an application-wide custom backend, call
`builder.provide(PasswordHasher::new(backend))` before building. Keep the hasher
registered and exported through `PasswordHasherCauldron` so the framework can
resolve its ownership. The supplied value takes precedence over the default.

The free `hash_password` and `validate_password` helpers always use the default
backend. Inject or retain a `PasswordHasher` instance when using a custom backend.
