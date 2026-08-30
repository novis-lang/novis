//! Novis's configuration: the directive registry today, and the `nvs.toml` tree that reads through
//! it as this milestone's later slices land.
//!
//! What a directive *is* — three fields, one of which is reloadability — lives in
//! [`directive`]'s own module doc, and the ADRs behind it are 0005 (the changeability class), 0078
//! § 2 (the reloadability field) and 0064 (the file format). Nothing in this crate restates them.
//!
//! [`mod@file`] reads one file of it. What is **not** here yet, in the order the milestone lands them:
//! the typed block tree that makes ADR 0064 § 3's unknown-key refusal real — [`file::parse`] is
//! generic over it and `toml::Table` is today's stand-in — then the tree of files (ADR 0103), the
//! per-app block (ADR 0104), and the immutable snapshot a request clones at start (ADR 0078 § 1).
//! Until those land `nvs-host` runs on compiled-in defaults and says so at each site.

pub mod directive;
pub mod file;

pub use directive::{Apply, Class, DIRECTIVES, Directive};
