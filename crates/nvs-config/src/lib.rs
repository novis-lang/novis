//! Novis's configuration: the directive registry, and the `nvs.toml` tree that reads through it.
//!
//! What a directive *is* — three fields, one of which is reloadability — lives in
//! [`directive`]'s own module doc, and the ADRs behind it are 0005 (the changeability class), 0078
//! § 2 (the reloadability field) and 0064 (the file format). Nothing in this crate restates them.
//!
//! [`mod@file`] reads one file of it and [`mod@tree`] is what it reads into: every block an ADR
//! states, with its own fields and an unknown key refused, which is `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`'s second refusal
//! made real. The two are separable on purpose — `file::parse` stays generic over the tree type, so
//! a caller that wants the untyped `toml::Table` for a `nvs config dump --origin` still has it.
//!
//! [`mod@resolve`] is the tree those files form: `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`'s root selection, `[[include]]` expansion
//! and one ordered stream where a later assignment wins **and is recorded**. That record is not a
//! log line — it is what makes later-wins acceptable in a file that grants capabilities, and its
//! module doc owns the reasoning.
//!
//! [`mod@trust`] is `rule:config/ownership-is-the-trust-boundary`, the boundary all of that rests on: no file the tree reads may be
//! writable by an account other than this one, and neither may the directory holding it. It reaches
//! the tree through [`resolve::Files::trust`] rather than being called from the resolver, because
//! whether there is a filesystem to ask is the reader's question and not the tree's.
//!
//! [`mod@secret`] is `rule:config/a-secret-is-a-file-whose-content-is-the-value`, which sits on that boundary rather than beside it: a
//! `password_file` names a file whose whole content is the value, and it is trusted exactly as the
//! file naming it was. It runs once over the flattened tree, because which of two `password_file`
//! assignments is in force is a question only the merge has answered.
//!
//! [`mod@app`] is `rule:config/an-application-is-its-entry-file-path` and `rule:config/every-matching-app-block-applies-least-specific-first`: which `[[app]]` blocks an entry file belongs to, and the one
//! effective block they fold into. It rests on the same canonicalization § 6 does —
//! [`trust::canonical`] is the only one, because a second would accept a `..` or a symlink the
//! other refuses — and its fold is [`resolve`]'s own `merge_table`, because § 2 is later-wins in a
//! different order rather than a second precedence rule.
//!
//! [`mod@snapshot`] is where all of that arrives: `rule:config/the-config-is-an-immutable-snapshot`'s immutable per-entry-file value a
//! request clones at start, and the holder a reload replaces whole. It is the one caller that has
//! both a [`Resolved`] and an entry file, so it is where [`mod@app`]'s per-app fold happens — over
//! the global tree, a block at a time, rather than through [`app::layer`], whose own module doc
//! says why.
//!
//! [`mod@mount`] is `rule:http-server/a-path-is-never-derived-from-a-url` and `rule:http-server/a-mount-table-expands-at-boot`, the one place a `*` meets a directory listing: the
//! `[[server.mount]]` blocks read into the literal set of entry files a server may execute. It is
//! split in two on purpose — the half that needs no disk runs inside [`server::validate`] with
//! every other block check, and the half that walks the tree is the server's own boot step, because
//! a `nvs run` of a CLI program should not fail over a document root this host does not have.
//!
//! [`mod@value`] is the other half of what a directive means: [`mod@tree`] answers which keys
//! exist, and this one answers what a value *is* — the size, duration or count it spells, and
//! whether one of them is within another. It is one parser because `rule:config/ini-set-is-core-config-set` says the boot path
//! and `Core\Config::set` share it, and it is directed by the unit the key takes because no
//! spelling can tell mega from minutes on its own.
//!
//! [`mod@request`] is the last step: one request's view of a [`Snapshot`], and the copy-on-write
//! overlay `rule:config/ini-set-is-core-config-set`'s `Core\Config::set` writes over it. It is a caller
//! [`mod@value`] exists to keep in agreement with the boot path — that path checks a ceiling out
//! of the file, this one checks the same ceiling out of the snapshot, and both go through
//! [`value::within_ceiling`]. `nvs-runtime`'s `Ctx` holds one per request; nothing in this crate
//! knows that, which is why the type is here and the field is there.
//!
//! What is **not** here yet: nothing reads a [`snapshot::Current`] — `nvs-host` still runs on
//! compiled-in defaults and says so at each site, so a limit a request set through
//! [`request::Request`] is visible to `Core\Config::get` and not yet to what enforces it. Stage 4
//! is where those meet.

pub mod app;
pub mod audit;
pub mod cache;
pub mod capability;
pub mod control;
pub mod db;
pub mod directive;
pub mod export;
pub mod file;
pub mod http;
pub mod log;
pub mod mode;
pub mod mount;
pub mod queue;
pub mod request;
pub mod resolve;
pub mod schedule;
pub mod secret;
pub mod server;
pub mod session;
pub mod snapshot;
pub mod store;
pub mod tree;
pub mod trust;
pub mod value;

pub use capability::{Cap, Scope};
pub use directive::{Apply, Class, DIRECTIVES, Directive};
pub use export::{DEFAULT_MAX_SERIES, Exporter, Metering, Tracing};
pub use request::Request;
pub use resolve::{Origin, Override, Resolved, Roots};
pub use server::{Capacity, ConnectionBounds, Waits};
pub use snapshot::{Current, Reload, Snapshot};
pub use tree::{Config, Setting};
pub use value::{Quantity, Unit};

/// The `nvs.toml` this project ships: every key [`mod@tree`] parses, commented out, under the prose
/// that says what it does. Its `[block]` headers are live, so setting a key is removing one `#`;
/// an `[[entry]]` header stays commented out, because an entry with no keys is refused.
///
/// It is inert by construction, and that is the point of it. A header with no key under it is
/// dropped by [`resolve()`](resolve::resolve), so a file in which no key is uncommented
/// resolves to what a host with no file at all resolves to
/// (`rule:config/no-configuration-file-is-a-complete-configuration`), so a project command can write
/// it into a directory without changing the run that takes it, and a default this project later
/// tightens still reaches the deployment that kept the file. What an operator gets is the roster:
/// the keys exist to be read, and uncommenting one is how they take ownership of that value.
///
/// Every setting line ends with a note saying what leaving the key unset does — `# default`,
/// `# default: <what unset gives>` or `# example` — because the value beside a commented-out key
/// does not say on its own whether it is the default or a suggestion. A key a running server
/// cannot apply a change to adds `; restart required`, and exactly the registry's
/// [`Apply::Boot`] rows do (`rule:config/reloadability-is-its-own-field`).
/// `bun nv directives --check-template` holds the file to the tree beside it — a key the
/// parser gained and the file omits, a key spelled twice, a key that parses and does nothing
/// without saying so, and a setting line whose note is missing, disagrees with another spelling of
/// the same field or shows a value this crate does not ship are each a failure there.
#[must_use]
pub fn default_file() -> &'static str {
    include_str!("default.toml")
}
