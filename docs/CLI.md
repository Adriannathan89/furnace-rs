# FURNACE CLI

FURNACE v0.9.2 provides Cargo-native execution, inspection, a minimal-project
generator, and a versioned JSON result
for finite FURNACE-owned commands. Human-readable output remains the default.

## Start a minimal HTTP application

Create an application outside an existing Cargo project:

```bash
furnace new my-app
cd my-app
furnace dev
```

`furnace new <name>` creates `./<name>` relative to the invocation directory. A
name starts with lowercase ASCII; its remaining characters may be lowercase
ASCII, digits, `-`, or `_`. Rust 2024 keywords and Cargo-reserved package names
are rejected. FURNACE preserves the supplied directory and package spelling.

The generated project contains exactly these seven files:

```text
<name>/
├── Cargo.toml
├── furnace.toml
└── src/
    ├── main.rs
    └── app/
        ├── mod.rs
        ├── routes.rs
        ├── controller.rs
        └── service.rs
```

The manifest starts the application at version `0.1.0`, uses edition 2024 and
Rust 1.94, and pins the installed FURNACE CLI version exactly. Its FURNACE dependency
uses `default-features = false` with only `http` and `runtime-tokio`; it has no
database, JWT, cookie, schema, migration, or authentication dependency. The
starter's `GET /` response is plain `Hello World!`.

`furnace.toml` contains:

```toml
[server]
host = "127.0.0.1"
port = 3000
```

The normal runtime overrides remain available: `FURNACE_SERVER__HOST` maps to
`server.host` and `FURNACE_SERVER__PORT` maps to `server.port`.

Generation validates arguments before writing, renders all files into a private
sibling staging directory, and publishes them with one atomic rename. An
existing destination, including an empty directory, is never changed. The
command does not download dependencies, run Cargo, initialize Git, select a
remote template, or ask an interactive question. It offers no template,
database, JWT, VCS, or target-directory option in v0.9. A successful human
result identifies the relative path and prints only `cd <name>` and `furnace dev`.

## Project and target selection

The CLI starts from the current working directory and resolves Cargo metadata.
With one eligible package and binary, selectors are unnecessary. Use
`--package <package>` (or `-p <package>`) and `--bin <binary>` when selection is
ambiguous:

```text
furnace run [--package <package>] [--bin <binary>] [-- <app-args>...]
furnace dev [--package <package>] [--bin <binary>] [-- <app-args>...]
furnace routes [--package <package>] [--bin <binary>]
furnace graph [--package <package>] [--bin <binary>]
furnace doctor [--package <package>] [--bin <binary>]
```

Cargo's ordinary single-package, `default-run`, and ambiguity behavior remains
authoritative. Arguments after `--` are forwarded only by `run` and `dev`;
inspection commands reject them.

## Output formats

The following finite commands accept `--format human|json`:

```text
furnace new <name>
furnace routes
furnace graph
furnace doctor
```

The option may appear once, before or after the command path. Both examples
are equivalent:

```bash
furnace --format json routes
furnace routes --format json
```

`human` is the default. `run`, `dev`, help, and version reject
`--format` because they are human/streaming interfaces. A duplicate, missing,
or unknown format value is CLI syntax failure `FURNACE204`.

In JSON mode stdout contains exactly one JSON document followed by one newline;
FURNACE writes no rendered warning or error text there. Cargo and rustc output
required to build inspection targets still passes through stderr. JSON paths use
`/` and are package-relative when possible.

Every document has this version-1 envelope:

```json
{
  "schema_version": 2,
  "command": "routes",
  "ok": true,
  "data": {},
  "diagnostics": []
}
```

`command` is the canonical spelling (`new`, `routes`, `graph`, `doctor`, `db
generate`, `db migrate`, `db rollback`, or `db status`) and is `null` only when
syntax cannot identify a command. `ok` is true only for exit-zero FURNACE-owned
completion. `data` is the command object, safe partial inspection data, or
`null`. `diagnostics` is an ordered list of FURNACE-owned records:

```json
{
  "severity": "error",
  "code": "FURNACE204",
  "title": "invalid command",
  "message": "...",
  "subject": null,
  "location": null,
  "suggestions": []
}
```

Severity is always `error` or `warning`; nullable `subject` and `location` are
intentional. Schema version 1 may add fields, and consumers must ignore unknown
object fields. Removing, renaming, changing the type of, or changing the
meaning of an existing field requires a new `schema_version`.

The finite schema owners are `new`, `routes`, `graph`, `doctor`, `db generate`,
`db migrate`, `db rollback`, and `db status`. A non-null source location has
one-based line and column numbers:

```json
{"file":"src/app/routes.rs","line":6,"column":5}
```

### JSON command data

`new` returns the project name, relative path, and this ordered file list:

```json
{
  "project_name": "my-app",
  "path": "my-app",
  "files": [
    "Cargo.toml",
    "furnace.toml",
    "src/main.rs",
    "src/app/mod.rs",
    "src/app/routes.rs",
    "src/app/controller.rs",
    "src/app/service.rs"
  ]
}
```

`routes` returns `{ "routes": [...] }`; every route record has `method`,
`path`, `route_trait`, `handler`, `controller`, `location`, and
`guard_active`. Route order remains method, path, controller, route trait, and
handler order. `graph` returns `root_cauldron`, `cauldrons`, `imports`,
`providers`, `dependencies`, and nullable `construction_order`. Cauldron records
contain `type_name`, `namespace`, and `location`; import records contain
`importer` and `imported`; providers retain `type_name`,
nullable owner and location, origin, visibility, and state; dependencies carry
`provider` and `dependency` names. `construction_order` is `null` when no valid
construction plan exists.

```json
{
  "routes": [{
    "method": "GET",
    "path": "/",
    "route_trait": "AppRoutes",
    "handler": "hello",
    "controller": "AppController",
    "location": {"file":"src/app/routes.rs","line":6,"column":5},
    "guard_active": false
  }]
}
```

```json
{
  "root_cauldron": "AppCauldron",
  "cauldrons": [{
    "type_name": "AppCauldron",
    "namespace": "crate::app",
    "location": {"file":"src/app/mod.rs","line":8,"column":1}
  }],
  "imports": [],
  "providers": [],
  "dependencies": [],
  "construction_order": []
}
```

`doctor` returns `{ "checks": [...] }`, where each check contains `group`,
`status`, and `summary`. Status is `pass`, `skipped`, `overridden`, or `failed`;
the existing group and summary ordering remains authoritative.

```json
{
  "checks": [{
    "group": "configuration",
    "status": "pass",
    "summary": "configuration sources are valid"
  }]
}
```

Invalid route or graph inspection retains every trustworthy record in `data`,
adds ordered error diagnostics, sets `ok` false, and exits 1. A failure before a
report exists or scaffold publication failure
uses `data: null`. JSON syntax failure requested through a recognized format
uses `FURNACE204`, `ok: false`, `data: null`, and exit 2.

## `furnace run` and `furnace dev`

`furnace run` builds the selected binary and forwards arguments after `--`. It
preserves an ordinary application exit status. `furnace dev` builds, supervises,
and watches the selected application's reachable workspace inputs. Changes are
debounced; a failed rebuild keeps the last good process when one is running.
Neither command wraps Cargo, rustc, or arbitrary application streams in JSON.

```bash
furnace run -- --seed-data
furnace run -p api --bin server -- --port 4000
furnace dev
furnace dev -p api --bin server -- --log=debug
```

## Inspection commands

`furnace routes`, `furnace graph`, and `furnace doctor` compile the selected standard
`Furnace::burn::<AppCauldron>()` application and obtain private inspection metadata
without normal provider construction, lifecycle startup, database connection,
migration, listener binding, or traffic serving. Human output remains the
existing table/section/check rendering; JSON exposes only the public schema
described above, never the private inspection protocol or its tokens.

## Persistence

The CLI does not manage database migrations. Applications can import
`furnace_rs_persistence::sea_orm::DatabaseCauldron` explicitly and use SeaORM's native
query and migration tools. See [persistence](furnace-rs-persistence.md).

## Diagnostics and exit codes

| Code | Meaning |
| --- | --- |
| 0 | Command completed successfully. |
| 1 | Build, Cargo resolution, inspection, scaffold filesystem, watcher, or other operational failure. |
| 2 | Invalid FURNACE CLI syntax, output-format selection, project name, or unsupported argument. |

`FURNACE204` identifies syntax or output-format failures. `FURNACE230` identifies
project-name, template rendering, staging, or publication failures. Existing
diagnostic families retain their meanings.

Operational diagnostics redact configuration values, credentials, URLs, private
inspection tokens, and arbitrary source error text. Human output is the default
compatibility surface; JSON is the stable machine-readable surface for the
finite commands only.
