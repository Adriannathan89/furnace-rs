# Preparing a stable release

Use this script to change only Cargo workspace/package version metadata for a
stable furnace-rs release.

## Prerequisites

- Run it from anywhere inside the furnace-rs Git repository.
- Install Bash, Python 3.11 or newer, Cargo, and Git.
- Ensure the target version has a completed `CHANGELOG.md` section named
  `## [X.Y.Z]` and that README/release documentation is current.

## Usage

```bash
script/release.sh 2.0.0
```

The script sets the workspace version to exactly `2.0.0`, updates every exact
internal FURNACE crate dependency to `=2.0.0`, updates matching FURNACE package
records in every `Cargo.lock`, then runs locked Cargo metadata and workspace
checks.

All fourteen distributable packages, including `furnace-rs-testing` and `furnace-rs-cli`, inherit the
workspace version. The script rejects a CLI manifest that pins its own version.

It does not edit README or changelog content, commit, tag, push, or publish.
Review the Cargo changes and complete the release documentation manually.
Independent examples' exact local FURNACE dependency pins are updated as well;
the examples retain their own application versions.

The [2.0.0 release guide](../docs/releases/2.0.0.md) consolidates the beta series
and documents migration and verification. For subsequent releases, select the
next version and follow the same gates.
After release review and authorization, committing and pushing the prepared
stable version to `main` activates the stable
publication workflow, which runs its release gates, publishes missing crate versions
to crates.io in dependency order, and creates the matching version Git tag and stable
GitHub Release. Configure `CRATES_IO_TOKEN` in the GitHub `stable` environment.

The workflow publishes `furnace-rs-testing` after `furnace-rs-common` and before `furnace-rs`.

Version updates apply to local workspace package records. Registry and Git
dependency records retain their published versions and checksums.

The gRPC macro companion publishes with the macro packages, and `furnace-rs-grpc`
publishes after common and before the facade. The unpublished protobuf test fixture
is excluded from publication and release version updates.

The WebSocket macro companion publishes with the macro packages, and
`furnace-rs-socket` publishes after common and before the facade. Both participate
in version preparation, nested lockfile updates, archive validation and release gates.
