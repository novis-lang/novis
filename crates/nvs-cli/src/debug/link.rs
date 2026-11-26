//! `nvs debug link`: the debug viewer's link, printed without starting the service
//! (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`).
//!
//! It reads `[debug] listen`, `--listen` and the token file as the service does
//! ([`super::secure::token`]), so the link is the one the next service admits. A missing token file
//! is created here exactly as the service creates it, in the data folder's private `debug/`, which
//! [`nvs_config::data::Folder::prepare_spool`] checks first, as the service does.
//!
//! **Whether a service runs** is whether `debug/service.lock` is held. The probe takes the lock and
//! lets go of it at once. A service that starts in that instant finds the lock taken, so the service
//! tries its lock again for a moment before it says another one runs ([`super::lock`]).
//!
//! `--json` prints `url`, `embed` (the link the editor frames, `rule:ide/the-extension-builds-no-ui-the-editor-already-has`),
//! `running` and `recording`. The language server answers the same four for the editor.
//!
//! **What it spends:** one read of the configuration and the token, and one lock probe.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use super::listen;

/// Print the link to the debug viewer.
///
/// The link contains the token that opens the viewer. This command starts nothing. It reads the
/// same settings as `nvs debug`, so the link works once `nvs debug` runs.
///
/// A note on standard error says when no `nvs debug` is running for the data folder, and when
/// `[debug] store` is off, because then no request is recorded.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// Print one JSON object with `url`, `embed`, `running` and `recording`. `embed` is the link
    /// an editor shows inside its own window.
    #[arg(long)]
    json: bool,
    /// The IP address and port that `nvs debug --listen` was given. Without it, the link uses
    /// `[debug] listen`.
    #[arg(long, value_name = "ADDRESS")]
    listen: Option<String>,
}

impl Args {
    /// Prints the link for the run's `--config` list.
    pub(crate) fn run(self, config: &[PathBuf]) -> ExitCode {
        let (_, mut listen, store) = match super::settings(config) {
            Ok(settings) => settings,
            Err(code) => return code,
        };
        if let Some(addr) = self.listen {
            listen.addr = addr;
        }
        let Ok(addr) = listen.addr.parse::<SocketAddr>() else {
            eprintln!(
                "error: `[debug] listen` is `{}`, and it must be an IP address and a port, \
                 such as `127.0.0.1:8137`",
                listen.addr
            );
            return ExitCode::FAILURE;
        };
        if addr.port() == 0 {
            eprintln!(
                "error: `[debug] listen` has the port `0`, so `nvs debug` picks a port when it \
                 starts. Use the link that `nvs debug` printed."
            );
            return ExitCode::FAILURE;
        }
        let Some(folder) = nvs_config::data::current() else {
            eprintln!("error: `nvs debug link` needs a data folder, and Novis cannot use one");
            return ExitCode::FAILURE;
        };
        if let Err(unusable) = folder.prepare_spool() {
            eprintln!(
                "error: `nvs debug link` cannot use the data folder: {}",
                unusable.reason()
            );
            if let Some(help) = unusable.help() {
                eprintln!("  help: {help}");
            }
            return ExitCode::FAILURE;
        }
        let [_, _, own_token] = folder.viewer_secrets();
        let token = match super::secure::token(&listen, &own_token) {
            Ok(token) => token,
            Err(message) => {
                eprintln!("error: {message}");
                return ExitCode::FAILURE;
            }
        };
        let lock_path = folder.debug().join("service.lock");
        let running = match super::probe(&lock_path) {
            Ok(running) => running,
            Err(err) => {
                eprintln!("error: cannot read `{}`: {err}", lock_path.display());
                return ExitCode::FAILURE;
            }
        };
        let (url, embed) = listen::links(addr, &token);
        if self.json {
            let out = serde_json::json!({
                "url": url,
                "embed": embed,
                "running": running,
                "recording": store,
            });
            println!("{out}");
        } else {
            println!("{url}");
        }
        if !running {
            eprintln!(
                "note: no `nvs debug` is running for the data folder `{}`. Start it with \
                 `nvs debug`. In development, `nvs serve` starts it.",
                folder.root().display()
            );
        }
        if !store {
            eprintln!(
                "note: `[debug] store` is off, so no request is recorded. Set `store = true` \
                 under `[debug]` in `nvs.toml`."
            );
        }
        ExitCode::SUCCESS
    }
}
