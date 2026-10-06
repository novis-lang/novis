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
//! [`offset_at`] and [`position_at`] are the whole boundary: a change of type
//! between LSP's `Position` and a byte offset, and no arithmetic.
//!
//! # What is answered
//!
//! The request set is closed: `rule:ide/the-request-set-is-closed` names every
//! standard request answered here, three code actions and four of Novis's own
//! — [`redactions`], [`regions`] and the two in [`imports`] — and
//! [`capabilities`] is the single place that declaration is written.
//! A request that is not on that list is a decision, not an addition — ADR 0099
//! § 3 holds the test a candidate has to pass, and ADR 0171 § 2 is what it
//! looks like applied to eight of them.
//!
//! # The open documents
//!
//! One open document is the unit of analysis and is its own entry point: its
//! `require`/`autoload` graph is resolved exactly as `nvs check` resolves it,
//! with every open buffer overlaid on the file under it, so a class edited in
//! one tab is the class another tab resolves against
//! (`rule:ide/an-open-document-is-its-own-entry-point`). [`Documents`] is that
//! store and [`analyse`] is the walk. Diagnostics are published only for what
//! is open, which is what keeps this a document server rather than M10's
//! workspace index.
//!
//! # The `.lspt` case format
//!
//! This is that format's one home. A `.lspt` case is a document, a cursor, a
//! request and the response rendered canonically
//! (`rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`), read by [`Case`],
//! rendered by [`Response`] and run by [`suite`] under `nvs lsp-test <paths>`:
//!
//! ```text
//! --TEST--
//! member completion survives an unclosed brace
//! --FILE--
//! <?nvs
//! class User { public string $name; }
//! var $u = new User();
//! $u-><|>
//! if (true) {
//! --REQUEST--
//! completion
//! --EXPECT--
//! name    property  string
//! ```
//!
//! | section | meaning |
//! |---|---|
//! | `--TEST--` | the one-line title, required |
//! | `--FILE--` | the document the request is asked about, required |
//! | `--FILE <relative/path>--` | another file beside it, any number of times |
//! | `--REQUEST--` | the question, one line, required |
//! | `--EXPECT--` | the frozen rendering, exact, required |
//!
//! Four names and no more. A `.lspt` case runs no program, so `.nvst`'s
//! `--ARGS--`, `--ENV--`, `--SKIPIF--`, `--CLEAN--` and oracle sections mean
//! nothing here and are refused rather than ignored. What the two formats share
//! is the line-oriented shape alone — `nvs_test::section` is the one lexer for
//! it, and sharing the *suite* would make `nvs test`'s count mean two things.
//!
//! `<|>` is the cursor: exactly one for a request asked at a position, and none
//! for one asked of the whole document, which [`Request::takes_cursor`] decides.
//! It is removed before the document is analysed and reported as a byte offset,
//! so what the server sees is what an editor holds.
//!
//! `--REQUEST--` is one line whose argument set is closed per request
//! (`rule:ide/a-request-line-is-closed`), and `--EXPECT--` is frozen: a case's
//! source may be corrected freely, its expectation may not be edited to make it
//! pass. The rendering it is compared against has one home, `nvs_lsp::render`
//! — a case that seems to need a spelling of its own has found a gap in that
//! module.

pub mod actions;
mod arguments;
mod capabilities;
pub mod card;
mod case;
pub mod completion;
pub mod completion_files;
pub mod coverage;
pub mod definition;
pub mod diagnostics;
pub mod directives;
mod document;
pub mod file_kinds;
pub mod folding;
pub mod hints;
pub mod hover;
pub mod html_template;
pub mod imports;
pub mod index;
pub mod links;
mod position;
pub mod redactions;
pub mod regions;
mod render;
pub mod selection;
pub mod semantic;
mod server;
pub mod settings;
pub mod split_join;
pub mod stubs;
pub mod suite;
pub mod symbols;

pub use capabilities::{
    CODE_ACTION_KINDS, SERVER_NAME, TOKEN_MODIFIERS, TOKEN_TYPES, declared_capabilities,
    initialize_result, negotiate_encoding, semantic_tokens_legend, server_capabilities,
    server_info, server_version,
};
pub use case::{AuxFile, CURSOR, Case, MAIN_PATH, ParseError, Request, RequestArgs};
pub use diagnostics::{Phases, SOURCE, for_document, phase_gated, to_wire};
pub use document::{
    Analysed, Document, Documents, analyse, analyse_current, analyse_file, path_of, uri_of,
};
pub use index::{
    CheckScope, Construction, DeclKind, Declaration, Import, Occurrence, Parameter, Refreshed,
    Site, SymbolIndex, Visibility, symbol_at,
};
pub use position::{encoding_of, offset_at, position_at, range_at, range_of};
pub use render::{Action, Link, Place, Redaction, Region, Response};
pub use server::{ServerError, run, serve};
pub use settings::{Client, Settings};
