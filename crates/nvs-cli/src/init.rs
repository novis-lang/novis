//! `nvs init` — the setup command: the data folder and the project's `nvs.toml`.
//!
//! The work itself is [`crate::config::init`], beside the rest of how a
//! configuration tree is found. This module owns only the command.

use std::path::PathBuf;
use std::process::ExitCode;

/// Set up Novis for one project.
///
/// `nvs init` creates the data folder. Then it writes a `nvs.toml` into the
/// working directory, or to the one path `--config` names. The folder for
/// `nvs.toml` must already exist. The command prints one line for each
/// path: `created`, `kept`, or an error with the command that fixes it.
///
/// You can run it again at any time. It creates nothing, never overwrites a
/// file, and checks both paths again.
///
/// Only your account and administrators may change the folder for
/// `nvs.toml`. If other accounts can change it, `nvs init` asks to restrict
/// it. Without a terminal, it prints the commands to run and exits with
/// status 1.
// `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`
// step 2's file, written on request. The implicit write a project command
// makes goes to the data folder instead.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// Restrict the folder for `nvs.toml` without asking, if other accounts
    /// can change it.
    #[arg(long, short = 'y')]
    yes: bool,
}

impl Args {
    /// Runs `nvs init`, writing to the one path the global `--config` names
    /// when it names one, with `data` the result of preparing the data folder.
    pub(crate) fn run(
        self,
        config: &[PathBuf],
        data: Result<
            (
                &'static nvs_config::data::Folder,
                nvs_config::data::Prepared,
            ),
            nvs_config::data::Unusable,
        >,
    ) -> ExitCode {
        crate::config::init(config, data, self.yes)
    }
}
