//! Command-line entry point for furnace-rs development tooling.
//!
//! Command behavior and user-facing documentation are provided by `furnace_rs_cli`.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    furnace_rs_cli::run()
}
