//! `nvs ext` — `rule:packaging/nvs-ext-is-the-authoring-tool`: the one tool an extension author
//! needs beside their own language's compiler, as six subcommands over one project layout.
//!
//! **What exists.** The command and its six subcommands parse, and each one is dispatched here.
//! A subcommand whose work is not built yet prints that to standard error and exits non-zero,
//! so a script that calls it fails rather than reading an empty success. A project's `nvsx.toml`
//! is read into its manifest by [`nvsx_toml`], whose module doc is the file's reference; `build`
//! is its caller. The packer and the load checks the built subcommands will call are
//! `nvs_ext::pack` and `nvs_ext::load`, whose module docs own what they check.
//!
//! **`nvs ext` never writes an `nvs.toml`.** It is not a project command in `initializes`'s
//! sense: `nvs ext test` runs with no configuration unless `--config` names one, so an extension
//! is tested with no grant, and the other subcommands read a project or a `.nvsx` and no
//! configuration at all.
//!
//! **Calls ADR 0246 leaves, settled here under AGENTS.md's priority ordering:**
//!
//! - `build` and `test` take the project directory as an optional argument and default to the
//!   working directory, the way `cargo` does, so the common case is the bare command.
//! - `inspect`, `verify` and `pin` take a `.nvsx` path and never a project, because each answers a
//!   question about a file that may have come from anywhere.

#[allow(
    dead_code,
    reason = "`nvs ext build` is the reader's caller, and its tests are its only one until then"
)]
mod nvsx_toml;

use std::process::ExitCode;

use crate::ExtCommand;

/// Runs one `nvs ext` subcommand.
pub(crate) fn run(command: ExtCommand) -> ExitCode {
    match command {
        ExtCommand::New { .. } => unbuilt("new"),
        ExtCommand::Build { .. } => unbuilt("build"),
        ExtCommand::Inspect { .. } => unbuilt("inspect"),
        ExtCommand::Test { .. } => unbuilt("test"),
        ExtCommand::Verify { .. } => unbuilt("verify"),
        ExtCommand::Pin { .. } => unbuilt("pin"),
    }
}

/// The answer of a subcommand this binary does not implement yet: one line on standard error and
/// a failing exit, so nothing mistakes it for a success.
fn unbuilt(subcommand: &str) -> ExitCode {
    eprintln!("error: `nvs ext {subcommand}` is not available in this version of `nvs`.");
    ExitCode::FAILURE
}
