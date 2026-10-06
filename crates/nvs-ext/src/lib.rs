//! Novis extensions: what a `.nvsx` carries, and the host that loads it.
//!
//! A `.nvsx` is one WebAssembly component with two custom sections, `nvs.manifest` and
//! `nvs.source` (`rule:packaging/an-nvsx-is-one-file-carrying-its-manifest`). This crate reads both
//! and, behind the `engine` feature, loads and runs the component under wasmtime.
//!
//! - [`section`] finds the two sections at a component's top level.
//! - [`manifest`] is the manifest's Rust type and its JSON parser.
//! - [`source`] is the `nvs.source` payload: the Novis source files the extension carries.
//! - [`types`] parses the Novis types a manifest writes, for the loader and the checker both.
//! - `load`, behind the `engine` feature, reads a `.nvsx` from its `[[extension]]` entry and
//!   refuses it, naming the entry, unless its pin, its component, its manifest and its imports all
//!   check. Its module doc owns the order and which Novis types the export check reads.
//! - `call`, behind the `engine` feature, runs a loaded extension: one instance per extension per
//!   request, a call the request's coroutine polls, and the request's CPU time and memory as its
//!   limits. Its module doc owns how a call waits, yields and fails.
//! - `convert`, behind the `engine` feature, turns a Novis value into the WIT value its manifest
//!   type crosses as, and back. Its module doc owns the value model and what it refuses.
//! - `handle`, behind the `engine` feature, is the per-call table a `mixed` argument is lent
//!   through, and the `value` accessors a guest reads it with.
//! - `wasi`, behind the `engine` feature, is the WASI every guest links, with an empty context.
//!   Its module doc owns what each interface gives a guest, and which interfaces are linked.
//! - `pack`, behind the `pack` feature, makes a `.nvsx` from a component or a core module, a
//!   manifest and source files, the same bytes each time.
//!
//! **The manifest model never links wasmtime.** The four modules above use `serde_json` and
//! `wasmparser` and nothing of the engine, because the checker and the language server read
//! manifests and never instantiate. They live here, in modules behind no feature, and wasmtime is
//! the optional dependency the default `engine` feature turns on. A reader that must not link the
//! engine — `nvs-types`, the language server — depends on this crate with
//! `default-features = false`. One crate keeps the manifest beside the loader that checks it; a
//! second crate for three modules would buy nothing the feature does not.
//!
//! Decisions ADR 0246 left to this crate, under the priority ordering:
//!
//! - **Two versions, kept apart.** The manifest's `"manifest": 1` is the version of the JSON format,
//!   read first, so a newer format is refused by its number and never as an unknown key. Its
//!   `"world"` is the `nvs:ext` version the component was built against, which load compares with
//!   the host's.
//! - **A method's WIT export is its name in kebab-case** ([`manifest::Method::export_name`]), the
//!   same mapping an enum's cases take, so the manifest has no second name to get wrong.
//! - **`nvs.source` is JSON too**: `{"source": 1, "files": [{"path": …, "text": …}]}`. Novis source
//!   is UTF-8 text, so a JSON string carries it whole, `nvs ext inspect --source` prints it as it
//!   is, and the crate has one parser for both sections. A path is relative, `/`-separated, never
//!   leaves the extension's directory, and names a `.nvs` file.
//! - **Every key is closed.** Both sections refuse a key they do not know, so a manifest written for
//!   a newer host fails on its format number or on the key, never by being half-read.

#[cfg(feature = "engine")]
pub mod call;
#[cfg(feature = "engine")]
pub mod convert;
#[cfg(feature = "engine")]
pub mod handle;
#[cfg(feature = "engine")]
pub mod load;
pub mod manifest;
#[cfg(feature = "pack")]
pub mod pack;
pub mod section;
pub mod source;
pub mod types;
#[cfg(feature = "engine")]
pub mod wasi;

use std::fmt;

/// Why a `.nvsx`'s sections or their payloads do not read, or why the packer cannot make one. The
/// message names the part that is wrong, and the loader adds which entry it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Malformed(pub String);

impl fmt::Display for Malformed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Malformed {}

/// The error for `message`.
fn malformed(message: impl Into<String>) -> Malformed {
    Malformed(message.into())
}

/// The WIT name of the Novis name `name`: kebab-case, so `distanceKm` and `distance_km` are both
/// `distance-km`.
fn kebab(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, c) in name.chars().enumerate() {
        if c == '_' {
            out.push('-');
        } else if c.is_ascii_uppercase() {
            if i > 0 && !out.ends_with('-') {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// Whether `name` is a Novis identifier: a letter or `_`, then letters, digits and `_`.
fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
