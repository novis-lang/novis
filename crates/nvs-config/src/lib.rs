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
//! What is **not** here yet, in the order the milestone lands them: the per-app block's matching and
//! layering (ADR 0104 § 2) and the immutable snapshot a request clones at start (ADR 0078 § 1).
//! Until those land `nvs-host` runs on compiled-in defaults and says so at each site.

pub mod directive;
pub mod file;
pub mod resolve;
pub mod tree;
pub mod trust;

pub use directive::{Apply, Class, DIRECTIVES, Directive};
pub use resolve::{Origin, Override, Resolved, Roots};
pub use tree::{Config, Setting};
