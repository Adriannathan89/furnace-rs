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

Adjust the path for your consumer. The module is public, but `hash_password` and
`validate_password` are currently private helpers. Applications cannot import
them yet; the contracts below describe their existing internal behavior.

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
