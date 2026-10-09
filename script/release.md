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
script/release.sh 1.0.1
```

The script sets the workspace version to exactly `1.0.1`, updates every exact
internal FURNACE crate dependency to `=1.0.1`, updates matching FURNACE package
records in every `Cargo.lock`, then runs locked Cargo metadata and workspace
checks.

All nine packages, including `furnace-rs-testing` and `furnace-rs-cli`, inherit the
workspace version. The script rejects a CLI manifest that pins its own version.

It does not edit README or changelog content, commit, tag, push, or publish.
Review the Cargo changes and complete the release documentation manually.

Version 1.0.1 is released; the command above illustrates its version update.
For subsequent releases, select the next version and follow the same gates. See
the [release guide](../docs/releases/1.0.1.md) for the recorded verification.
After release review and authorization, committing and pushing the prepared
stable version to `main` activates the stable
publication workflow, which runs its release gates, publishes missing crate versions
to crates.io in dependency order, and creates the `v1.0.1` Git tag and stable
GitHub Release. Configure `CRATES_IO_TOKEN` in the GitHub `stable` environment.

The workflow publishes `furnace-rs-testing` after `furnace-rs-common` and before `furnace-rs`.

Version updates apply to local workspace package records. Registry and Git
dependency records retain their published versions and checksums.
