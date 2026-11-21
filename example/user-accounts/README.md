# User accounts: registration, password hashing, and login

This independent FURNACE 2.0.0 project is a complete accounts feature: it
registers users, stores Argon2 password hashes, issues JWT access tokens, and
protects a profile route. It needs no database; accounts live in memory.

[Step 4](../../docs/tutorial/04-accounts-and-login.md) and
[step 5](../../docs/tutorial/05-testing.md) of the learning path explain this
code piece by piece.

## Run

Requires Rust 1.94 or newer. From this directory:

```sh
cp .env.example .env
cargo run
```

`.env` supplies the JWT signing secret through `furnace.toml` interpolation.
The secret must contain at least 32 bytes for HS256.

## Requests

```sh
curl -i -X POST http://127.0.0.1:3005/accounts/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"ada","password":"correct horse battery staple"}'

curl -i -X POST http://127.0.0.1:3005/accounts/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"ada","password":"correct horse battery staple"}'

curl -i http://127.0.0.1:3005/accounts/me \
  -H 'Authorization: Bearer YOUR_ACCESS_TOKEN'
```

| Request | Response |
| --- | --- |
| Register a new username | `200` with `{"id":1,"username":"ada"}` |
| Register the same username again | `409` `conflict` |
| Username under 3 or password under 12 characters | `422` `validation_error` |
| Login with a wrong password or an unknown username | `401` `invalid credentials` for both |
| `GET /accounts/me` with the access token | `200` with the account |
| `GET /accounts/me` without a token | `401` `authentication was rejected` |

## Test

```sh
cargo test
```

The tests run in process with no listener and no `.env`. They supply a JWT
service built from in-memory configuration and a password hasher with minimal
Argon2 costs.

## Where to look

| File | Shows |
| --- | --- |
| `src/accounts/mod.rs` | One cauldron listing the feature's imports, providers, and controllers |
| `src/accounts/repository.rs` | A hand-written `Injector` and an atomic check-and-insert |
| `src/accounts/service.rs` | `hash_password_async`, `validate_optional_password_async`, JWT signing, and the Passport strategy |
| `src/accounts/controller.rs` | Validated input, error mapping, a public controller, and a guarded one |
| `src/accounts/tests.rs` | Service and controller fixtures with supplied providers |

## Security notes

- Passwords are hashed with Argon2id on Tokio's blocking pool, so hashing does
  not stall request handling.
- Login verifies against a decoy hash when the username is unknown and returns
  one message for both failures, so neither timing nor the response reveals
  which usernames exist.
- The password length limit bounds the work one request can ask for.

This example has no rate limiting, token revocation, refresh tokens, or durable
storage. A real application owns those.

Compare with [protected-route](../protected-route/), which focuses on
trait-bound services and uses a fixed demo credential instead of hashing.
