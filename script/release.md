# Preparing a stable release

Use this script to change only Cargo workspace/package version metadata for a
stable MADS.rs release.

## Prerequisites

- Run it from anywhere inside the MADS.rs Git repository.
- Install Bash, Python 3.11 or newer, Cargo, and Git.
- Ensure the target version has a completed `CHANGELOG.md` section named
  `## [X.Y.Z]` and that README/release documentation is current.

## Usage

```bash
script/release.sh 0.9.2
```

The script sets the workspace version to exactly `0.9.2`, updates every exact
internal MADS crate dependency to `=0.9.2`, updates matching MADS package
records in every `Cargo.lock`, then runs locked Cargo metadata and workspace
checks.

All nine packages, including `mads-testing` and `mads-cli`, inherit the
workspace version. The script rejects a CLI manifest that pins its own version.

It does not edit README or changelog content, commit, tag, push, or publish.
Review the Cargo changes and complete the release documentation manually.

After committing the prepared stable version, push it to `main`. The stable
publication workflow runs all release gates, publishes missing crate versions
to crates.io in dependency order, and creates the `v0.9.2` Git tag and stable
GitHub Release. Configure `CRATES_IO_TOKEN` in the GitHub `stable` environment.

The workflow publishes `mads-testing` after `mads-common` and before `mads`.

Version updates apply to local workspace package records. Registry and Git
dependency records retain their published versions and checksums.
