//! `nvs init` — the shipped default `nvs.toml`, written on request.
//!
//! The write itself is [`crate::config::init`], beside the rest of how a
//! configuration tree is found. This module owns only the command.

use std::path::PathBuf;
use std::process::ExitCode;

/// Write the shipped default `nvs.toml` into the working directory, or
/// to the one path `--config` names.
///
/// The directory the file goes into must already exist. It and the
/// directory containing it must both pass the ownership check, because
/// this is a file `nvs serve` will read as configuration.
///
/// Use this to start a configuration for one project. An existing file is
/// never overwritten. If the file cannot be written, the command stops
/// with an error.
// `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`
// step 2's file, written on request. The implicit write a project command
// makes goes to the data folder instead.
#[derive(clap::Args)]
pub(crate) struct Args {}

impl Args {
    /// Runs `nvs init`, writing to the one path the global `--config` names
    /// when it names one.
    pub(crate) fn run(self, config: &[PathBuf]) -> ExitCode {
        crate::config::init(config)
    }
}
