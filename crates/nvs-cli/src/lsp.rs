//! `nvs lsp` — the language server, on standard input and output.
//!
//! The server itself is `nvs_lsp`'s. This module owns only the command's
//! arguments and where a failure is reported.

use std::process::ExitCode;

/// Speak the Language Server Protocol on standard input and output.
///
/// Started by an editor, not by a person: it reads LSP 3.17 frames on
/// stdin and writes them on stdout, so a terminal that runs it sees
/// nothing and appears to hang. It is the one implementation of Novis's
/// language smarts, and every editor client is a shell around it.
///
/// Configures nothing. Everything configurable arrives in `initialize`
/// and in `workspace/didChangeConfiguration`, because the editor is what
/// owns the settings and a flag here would be a second, staler copy.
///
/// `--stdio` is the one exception, and it configures nothing either: it is
/// the convention a language client appends to say which transport it
/// chose, and stdio is the only transport this server has. Clients append
/// it without being asked — `vscode-languageclient` does so for any
/// `Executable` whose `transport` is set — and a server that rejected it
/// would exit before reading a frame, which reaches the user as an editor
/// with no language features rather than as an argument error.
// `rule:ide/one-server-two-thin-clients`.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// Accepted and ignored, so a client that names its transport is not
    /// refused for it.
    #[arg(long)]
    stdio: bool,
}

impl Args {
    /// Runs `nvs lsp` with the arguments clap parsed.
    pub(crate) fn run(self) -> ExitCode {
        // `stdio` names the transport the client chose and there is no other
        // one to choose, so it selects nothing here.
        let _ = self.stdio;
        match nvs_lsp::run() {
            Ok(()) => ExitCode::SUCCESS,
            // stderr, and never stdout: the writer thread owns stdout and a
            // line printed there would be read as a protocol frame
            // (`rule:ide/stdout-belongs-to-the-protocol`). By the time this is
            // reached the client is usually gone, so this is for the editor's
            // server log rather than for anyone watching.
            Err(err) => {
                eprintln!("error: {err}");
                ExitCode::FAILURE
            }
        }
    }
}
