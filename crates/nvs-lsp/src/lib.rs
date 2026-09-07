//! `nvs lsp` — the Novis language server.
//!
//! LSP 3.17 over stdio, **synchronous**: `lsp_server::Connection::stdio()`
//! spawns one reader thread and one writer thread and hands back a pair of
//! channel endpoints, and everything else happens on the thread that called it.
//! That is `rule:ide/the-server-is-synchronous` in full — there is no executor,
//! no task type and no `Future` anywhere in this crate's graph, so ADR 0072's
//! "one scheduler, and it is `nvs-host`'s" is not even a question here.
//!
//! **stdout belongs to the protocol.** The writer thread owns this process's
//! standard output, so anything else that prints a byte to it desynchronises
//! the framing and the client's next `Content-Length` read lands in the middle
//! of a message. The symptom is a server that stops answering with no error
//! anywhere, which is why `rule:ide/stdout-belongs-to-the-protocol` is a rule
//! rather than a habit: this crate takes no `clippy::print_stdout` allowance,
//! and `tests/stdout_policy.rs` extends the same claim to every crate the
//! server links, since a lint only covers the crate it is written in.
//! Logging goes to stderr and to `window/logMessage`.
//!
//! **Position arithmetic has one home and it is not this crate.**
//! `rule:ide/positions-have-one-home` puts every offset-to-position conversion
//! in `nvs-diagnostics` — `line_col`, `utf16_col`, `offset_of` — and this crate
//! negotiates the encoding and then calls them. A conversion written here would
//! be a second implementation of the arithmetic an editor's whole cursor
//! behaviour rests on, which is the bug that rule exists to prevent.
//!
//! # What is answered
//!
//! The request set is closed: `rule:ide/the-request-set-is-closed` names nine
//! standard requests, two code actions and exactly one of Novis's own, and
//! [`server_capabilities`] is the single place that declaration is written.
//! A tenth request is a decision, not an addition — ADR 0099 § 3
//! holds the test a candidate has to pass.

mod capabilities;
mod server;

pub use capabilities::{
    CODE_ACTION_KINDS, SERVER_NAME, TOKEN_MODIFIERS, TOKEN_TYPES, initialize_result,
    negotiate_encoding, semantic_tokens_legend, server_capabilities, server_info, server_version,
};
pub use server::{ServerError, run, serve};
