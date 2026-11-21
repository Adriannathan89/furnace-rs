# 3. Configuration and validation

Previous: [Providers and cauldrons](02-providers-and-cauldrons.md) · [Learning path](README.md) · Next: [Accounts and login](04-accounts-and-login.md)

Two kinds of outside data reach an application: settings chosen by whoever
deploys it, and input sent by whoever calls it. This step gives each a typed
struct and lets the framework reject bad values before your code runs.

## The program

Replace `src/main.rs`:

```rust
use furnace_rs::prelude::*;
use serde::{Deserialize, Serialize};

// Typed settings read from the `[greeting]` table of `furnace.toml`.
#[derive(Clone, Configuration)]
#[config(prefix = "greeting")]
struct GreetingConfig {
    #[config(default = "Hello")]
    salutation: String,
    #[config(default = 3, validate(range(min = 1, max = 10)))]
    max_names: u8,
    // Printed as `[REDACTED]` by `Debug` and `Display`.
    signature: Option<Secret<String>>,
}

impl Injector for GreetingConfig {
    type Dependencies = (Config,);

    async fn inject((config,): Self::Dependencies) -> furnace_rs::core::Result<Self> {
        Ok(config.parse()?)
    }
}

// Request input: deserialized by Serde, then checked by `Input`.
#[derive(Deserialize, Input)]
struct GreetRequest {
    #[validate(nonempty)]
    names: Vec<String>,
}

#[derive(Serialize)]
struct GreetResponse {
    greetings: Vec<String>,
    signed: bool,
}

#[controller]
struct GreetingController {
    #[inject]
    config: GreetingConfig,
}

#[controller(route = "/greetings")]
impl GreetingController {
    #[post]
    async fn greet(
        &self,
        ValidatedJson(input): ValidatedJson<GreetRequest>,
    ) -> HttpResult<Json<GreetResponse>> {
        // A rule that depends on configuration is checked in the handler.
        if input.names.len() > usize::from(self.config.max_names) {
            return Err(BadRequest::new("too many names").into());
        }

        Ok(Json(GreetResponse {
            greetings: input
                .names
                .iter()
                .map(|name| format!("{}, {name}!", self.config.salutation))
                .collect(),
            signed: self.config.signature.is_some(),
        }))
    }
}

#[cauldron(root, providers = [GreetingConfig], controllers = [GreetingController])]
struct AppCauldron;

#[furnace_rs::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
```

Add a table to `furnace.toml`:

```toml
[greeting]
salutation = "Welcome"
max_names = 2
```

## Run it

```sh
cargo run
curl -X POST http://127.0.0.1:3000/greetings \
  -H 'Content-Type: application/json' -d '{"names":["Ada"]}'
# {"greetings":["Welcome, Ada!"],"signed":false}
```

An empty list never reaches the handler:

```sh
curl -X POST http://127.0.0.1:3000/greetings \
  -H 'Content-Type: application/json' -d '{"names":[]}'
```

```json
{
  "error": {
    "code": "validation_error",
    "message": "input validation failed",
    "issues": [
      { "source": "body", "path": ["names"], "code": "too_small", "message": "value is too small" }
    ]
  }
}
```

The response has status 422. Issues name the field and the rule, never the
rejected value.

## Configuration

| Attribute | Effect |
| --- | --- |
| `#[config(prefix = "greeting")]` | Reads keys under the `[greeting]` table. |
| `#[config(default = ...)]` | Used when the key is absent. A present but invalid value is an error, not a fallback. |
| `#[config(validate(...))]` | Checks the parsed value. |
| `#[config(rename = "...")]` | Reads a differently named key. |
| `Option<T>` | `None` when the key is absent. |
| `Secret<T>` | Hides the value from `Debug` and `Display`; read it with `.expose()`. Secrets cannot have defaults. |

Configuration is checked at startup. Set `max_names = 50` and run again: the
process exits with a `FURNACE006` "provider construction failed" diagnostic for
`GreetingConfig` before the listener opens, so a bad deployment never serves
traffic.

Keep secrets out of `furnace.toml` by interpolating them:

```toml
[greeting]
signature = "${GREETING_SIGNATURE}"
```

`${NAME}` is filled from the process environment first, then from a local
`.env` file that you do not commit.

## Validation

| Extractor | Validates | Issue `source` |
| --- | --- | --- |
| `ValidatedJson<T>` | The JSON body | `body` |
| `ValidatedQuery<T>` | The query string | `query` |
| `ValidatedPath<T>` | Path parameters | `path` |

Common field rules: `nonempty`, `email`, `length(min = N, max = N)`,
`length(exact = N)`, `range(min = N)`, `range(max = N)`, `positive`, `negative`,
`multiple_of = N`, `required`, `nested`, and `custom = function`. The plain
`Json`, `Query`, and `Path` extractors deserialize without running `Input`.

## Errors

Return `HttpResult<T>` and convert one of the standard errors with `.into()` or
`?`. All of them render the same `{ "error": { "code", "message" } }` envelope.

| Type | Status | Message |
| --- | ---: | --- |
| `BadRequest::new(message)` | 400 | Yours |
| `Unauthorized::new(message)` | 401 | Yours |
| `Forbidden::new(message)` | 403 | Yours |
| `NotFound::new(message)` | 404 | Yours |
| `Conflict::new(message)` | 409 | Yours |
| `InternalError::new(source)` | 500 | Always `internal server error`; the source stays on the server |

Wrap unexpected failures in `InternalError` so their details are never sent to
the client.

## Try it

- Send three names with `max_names = 2` and read the 400 response.
- Add `#[validate(email)] reply_to: String` to the request.
- Set `GREETING_SIGNATURE` in the environment, add the interpolated key above,
  and watch `signed` become `true`.

Reference: [root README](../../README.md#typed-configuration-and-secrets)
for every supported field type and validator.

Next: [Accounts and login](04-accounts-and-login.md)
