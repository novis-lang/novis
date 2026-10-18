//! `nvs stubs` — the declaration files for `Core`, written to a directory.
//!
//! The generator is `nvs_lsp::stubs`, and this command is its second caller:
//! the language server writes the same files by itself.

use std::path::PathBuf;
use std::process::ExitCode;

/// Write one declaration file per `Core` class, enum and attribute.
///
/// An editor opens these files when you jump to a `Core` name. Each file
/// has the real signatures and the reference cards, and empty bodies. The
/// language server writes the same files by itself; this command writes
/// them for any other reader.
// `rule:ide/the-stub-tree-is-where-core-is-declared`.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// The directory the files are written to, created if it is absent.
    #[arg(long, default_value = "stubs")]
    out: PathBuf,
}

impl Args {
    /// Runs `nvs stubs` with the arguments clap parsed.
    pub(crate) fn run(self) -> ExitCode {
        match nvs_lsp::stubs::write_to(&self.out) {
            Ok(written) => {
                for path in written {
                    println!("{}", path.display());
                }
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("error: could not write {}: {err}", self.out.display());
                ExitCode::FAILURE
            }
        }
    }
}
