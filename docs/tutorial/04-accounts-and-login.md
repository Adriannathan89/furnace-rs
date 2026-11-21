# 4. Accounts and login

Previous: [Configuration and validation](03-configuration-and-validation.md) · [Learning path](README.md) · Next: [Testing](05-testing.md)

This step reads a complete feature instead of one file: the runnable
[user-accounts example](../../example/user-accounts/). It registers accounts,
stores Argon2 password hashes, issues JWT access tokens, and protects a profile
route. Open the example beside this page.

```sh
cd example/user-accounts
cp .env.example .env
cargo run
```

## Layout

```text
src/
├── main.rs                 root cauldron and entry point
└── accounts/
    ├── mod.rs              AccountsCauldron: everything the feature registers
    ├── model.rs            request, response, claims, and principal types
    ├── repository.rs       where accounts are stored
    ├── service.rs          use cases and the JWT strategy
    ├── controller.rs       HTTP endpoints and the guard
    └── tests.rs            in-process tests (step 5)
```

The feature needs two more facade features than the earlier steps:

```toml
furnace-rs = { version = "=2.0.0", default-features = false, features = ["http", "jwt", "password-hasher", "runtime-tokio"] }
```

## The request flow

```text
POST /accounts/register ─▶ ValidatedJson ─▶ AccountService::register ─▶ hash ─▶ repository
POST /accounts/login    ─▶ ValidatedJson ─▶ AccountService::authenticate ─▶ verify ─▶ sign JWT
GET  /accounts/me       ─▶ MemberGuard ─▶ verify JWT ─▶ AccountJwtStrategy ─▶ handler
```

## Registering: store a hash, never the password

`service.rs`:

```rust
pub async fn register(
    &self,
    username: &str,
    password: &str,
) -> PasswordHasherResult<Option<Account>> {
    let password_hash = self.hasher.hash_password_async(password).await?;

    Ok(self.repository.insert(username, password_hash))
}
```

- `PasswordHasher` is injected like any provider. It comes from
  `PasswordHasherCauldron`, which `AccountsCauldron` imports.
- The result is a PHC string such as `$argon2id$v=19$m=19456,t=2,p=1$...`. It
  carries its own salt and parameters, so one text column is all you store.
- Use the `_async` methods in handlers. Argon2 is deliberately slow; they run it
  off the runtime threads so other requests keep moving.
- The repository checks and inserts under one lock and returns `None` for a
  taken username, which the controller turns into `409 Conflict`.

## Logging in: one answer for every failure

```rust
pub async fn authenticate(
    &self,
    username: &str,
    password: &str,
) -> PasswordHasherResult<Option<Account>> {
    let account = self.repository.find_by_username(username);
    let stored_hash = account.as_ref().map(|account| account.password_hash.as_str());

    let matches = self
        .hasher
        .validate_optional_password_async(password, stored_hash)
        .await?;

    Ok(account.filter(|_| matches))
}
```

The lookup result goes straight into `validate_optional_password_async`. When
the account does not exist it still spends one verification, so the response
time does not show which usernames are registered. The controller then returns
the same `401` body for an unknown username and a wrong password:

```rust
.ok_or_else(|| Unauthorized::new("invalid credentials"))?;
```

Avoid the shorter version that returns early when the lookup fails. It answers
faster for unknown usernames and tells an attacker which accounts exist.

## Issuing a token

```rust
pub fn issue_token(&self, account: &Account) -> JwtResult<String> {
    self.jwt.sign(
        AccountClaims { account_id: account.id },
        JwtSignOptions::access(Duration::from_secs(15 * 60)).subject(account.id.to_string()),
    )
}
```

`JwtService` reads its key from the `[passport]` table:

```toml
[passport]
secret = "${JWT_SECRET}"
algorithms = ["HS256"]
```

The secret comes from `.env` locally and from the process environment in
production. HS256 needs at least 32 bytes; a shorter secret stops startup.

## Protecting a route

Three pieces work together.

**A principal** is what handlers receive after authentication:

```rust
#[derive(PassportPrincipal)]
pub struct AccountPrincipal {
    pub id: u64,
    pub username: String,
    #[roles]
    pub roles: Vec<String>,
}
```

**A strategy** turns verified claims into that principal. The framework has
already checked the signature, expiry, and token kind when `validate` runs:

```rust
#[passport_strategy(name = "jwt")]
impl PassportStrategy for AccountJwtStrategy {
    type Claims = AccountClaims;
    type Principal = AccountPrincipal;
    const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;

    async fn validate(
        &self,
        _context: &PassportContext<'_>,
        claims: &JwtClaims<Self::Claims>,
    ) -> PassportResult<Self::Principal> {
        self.accounts
            .find_by_id(claims.custom.account_id)
            .map(|account| AccountPrincipal {
                id: account.id,
                username: account.username,
                roles: vec!["member".to_owned()],
            })
            .ok_or_else(PassportError::reject)
    }
}
```

**A guard** names the strategy, the token source, and the policy, and a
controller opts in through `Sealable`:

```rust
#[guard(strategy = "jwt", principal = AccountPrincipal, source = bearer, roles(any = ["member"]))]
struct MemberGuard;

impl Sealable for ProfileController {
    fn seals() -> SealRegistration<Self> {
        Self::seal::<MemberGuard>()
    }
}

#[controller(route = "/accounts")]
impl ProfileController {
    #[get("/me")]
    async fn me(
        &self,
        principal: Authenticated<AccountPrincipal>,
    ) -> HttpResult<Json<AccountResponse>> {
        Ok(Json(AccountResponse {
            id: principal.id,
            username: principal.username.clone(),
        }))
    }
}
```

A controller without `impl Sealable` is public, which is why registration and
login live in a separate `AccountController`. To mix both in one controller,
seal it and mark the public methods `#[seal(skip)]`.

| Outcome | Status |
| --- | ---: |
| Missing, malformed, expired, or rejected token | 401 |
| Valid token that fails `roles`, `permissions`, or a predicate | 403 |
| Valid token and policy | handler runs |

## Try it

```sh
curl -X POST http://127.0.0.1:3005/accounts/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"ada","password":"correct horse battery staple"}'
# {"id":1,"username":"ada"}

curl -X POST http://127.0.0.1:3005/accounts/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"ada","password":"correct horse battery staple"}'
# {"access_token":"..."}

curl http://127.0.0.1:3005/accounts/me -H 'Authorization: Bearer YOUR_ACCESS_TOKEN'
# {"id":1,"username":"ada"}
```

Then register the same username again (409), send an eight-character password
(422), log in as `nobody` (401), and call `/accounts/me` without a header
(401).

## What the example leaves to you

Accounts live in memory and vanish on restart. A real application also owns
rate limiting for login attempts, token revocation and refresh, and password
policy beyond length. See the [security policy](../SECURITY.md) for the full
list of application responsibilities.

Reference: [Password hasher](../guides/password-hasher.md) ·
[Passport and JWT](../../README.md#passport-configuration-and-jwt-profiles) ·
[protected-route example](../../example/protected-route/) for trait-bound
services

Next: [Testing](05-testing.md)
