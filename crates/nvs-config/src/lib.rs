//! Novis's configuration: the directive registry today, and the `nvs.toml` tree that reads through
//! it as this milestone's later slices land.
//!
//! What a directive *is* — three fields, one of which is reloadability — lives in
//! [`directive`]'s own module doc, and the ADRs behind it are 0005 (the changeability class), 0078
//! § 2 (the reloadability field) and 0064 (the file format). Nothing in this crate restates them.
//!
//! [`mod@file`] reads one file of it and [`mod@tree`] is what it reads into: every block an ADR
//! states, with its own fields and an unknown key refused, which is ADR 0064 § 3's second refusal
//! made real. The two are separable on purpose — `file::parse` stays generic over the tree type, so
//! a caller that wants the untyped `toml::Table` for a `nvs config dump --origin` still has it.
//!
//! [`mod@resolve`] is the tree those files form: ADR 0103's root selection, `[[include]]` expansion
//! and one ordered stream where a later assignment wins **and is recorded**. That record is not a
//! log line — it is what makes later-wins acceptable in a file that grants capabilities, and its
//! module doc owns the reasoning.
//!
//! [`mod@trust`] is ADR 0103 § 6, the boundary all of that rests on: no file the tree reads may be
//! writable by an account other than this one, and neither may the directory holding it. It reaches
//! the tree through [`resolve::Files::trust`] rather than being called from the resolver, because
//! whether there is a filesystem to ask is the reader's question and not the tree's.
//!
//! [`mod@secret`] is ADR 0103 § 7, which sits on that boundary rather than beside it: a
//! `password_file` names a file whose whole content is the value, and it is trusted exactly as the
//! file naming it was. It runs once over the flattened tree, because which of two `password_file`
//! assignments is in force is a question only the merge has answered.
//!
//! [`mod@app`] is ADR 0104 §§ 1-2: which `[[app]]` blocks an entry file belongs to, and the one
//! effective block they fold into. It rests on the same canonicalization § 6 does —
//! [`trust::canonical`] is the only one, because a second would accept a `..` or a symlink the
//! other refuses — and its fold is [`resolve`]'s own `merge_table`, because § 2 is later-wins in a
//! different order rather than a second precedence rule.
//!
//! [`mod@snapshot`] is where all of that arrives: ADR 0078 § 1's immutable per-entry-file value a
//! request clones at start, and the holder a reload replaces whole. It is the one caller that has
//! both a [`Resolved`] and an entry file, so it is where [`mod@app`]'s per-app fold happens — over
//! the global tree, a block at a time, rather than through [`app::layer`], whose own module doc
//! says why.
//!
//! [`mod@value`] is the other half of what a directive means: [`mod@tree`] answers which keys
//! exist, and this one answers what a value *is* — the size, duration or count it spells, and
//! whether one of them is within another. It is one parser because ADR 0064 § 5 says the boot path
//! and `Core\Config::set` share it, and it is directed by the unit the key takes because no
//! spelling can tell mega from minutes on its own.
//!
//! What is **not** here yet: nothing reads a [`snapshot::Current`] — `nvs-host` still runs on
//! compiled-in defaults and says so at each site, and `Core\Config::get`/`set` (ADR 0064 § 5) are
//! unwritten, so [`mod@value`] has neither of the two callers it exists to keep in agreement: the
//! boot-time ceiling check is the next slice and the request-time one waits on those members.

pub mod app;
pub mod directive;
pub mod file;
pub mod resolve;
pub mod secret;
pub mod snapshot;
pub mod tree;
pub mod trust;
pub mod value;

pub use directive::{Apply, Class, DIRECTIVES, Directive};
pub use resolve::{Origin, Override, Resolved, Roots};
pub use snapshot::{Current, Reload, Snapshot};
pub use tree::{Config, Setting};
pub use value::{Quantity, Unit};
