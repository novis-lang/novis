//! [ADR 0077](../../../../docs/adr/0077-compile-time-routing.md) § 1's
//! `#[Route]`: what one route declaration may carry.
//!
//! # Why this is a recognized name rather than a shape alias
//!
//! § 1 writes the attribute as an ordinary
//! [ADR 0046](../../../../docs/adr/0046-attributes-shape-literal-metadata.md)
//! `type Core\Route = {path: string, method: Core\Http\Method, name?: string};`
//! and then adds the one thing that makes it not one: the compiler acts on the
//! attribute only when its name **resolves** to `Core\Route`, so a userland
//! `type Route = {…};` is not it however it is spelled and a framework carrying
//! its own `Route`-shaped literal does not contribute a route. That is
//! [ADR 0071](../../../../docs/adr/0071-derived-codecs.md) § 1's rule, so the
//! name sits on [`crate::derive::ATTRIBUTES`] and is matched *nominally* after
//! [`nvs_hir::resolve_ref`] — and, being matched nominally, it names no shape,
//! which is why what it may hold is the roster below rather than an alias
//! lookup.
//!
//! # What is checked here, and what the table still owes
//!
//! Checked: the payload's field names against [`OPTIONS`], each value's type,
//! and a field given twice — [`crate::attributes::check_roster`], the walk
//! every recognized name with a payload shares. That is one declaration read on
//! its own, which is all this pass can see.
//!
//! # Known gaps
//!
//! 1. **The route table itself is not built**, so none of § 1-§ 3's four
//!    compile errors is reported: a duplicate route or duplicate `name` (a
//!    question about the whole program's enumeration), a `{param}` with no
//!    matching method parameter and a capture whose parameter type has no
//!    conversion from a segment (questions about the method the attribute is
//!    attached to, which this walk does not carry). § 2's path grammar is
//!    validated by that same pass, so a malformed `path` is admitted here too.
//! 2. **Nothing requires `path` and `method`.** § 1's shape marks only `name`
//!    optional, so an empty `#[Route]` is a payload this pass admits and the
//!    userland alias it replaces would not. It is the table pass that needs
//!    both to build a row, and it is what should refuse a row missing either —
//!    the same reading [`crate::commands`]' gap 2 gives `#[Command]`'s `name`,
//!    for the same reason: one rule, one home.

use crate::testing::OptionTy;

/// `#[Route(path: string, method: Core\Http\Method, name?: string)]` — ADR 0077
/// § 1's own spelling, in the order that section writes it.
///
/// `method` is an enum case rather than a string
/// ([ADR 0063](../../../../docs/adr/0063-core-api-conventions.md) R11), and an
/// enum case is one of the three things a payload may contain (ADR 0046 § 2);
/// it is the same `Core\Http\Method` `Core\Request::method` answers with, which
/// is why the row names that enum rather than a spelling of its own. There is
/// no `methods:` row: § 1 serves two verbs by repeating the attribute
/// (ADR 0046 § 3), so a union or an array here would be a second way to write
/// what the existing rule already covers.
pub(crate) const OPTIONS: &[(&str, OptionTy)] = &[
    ("path", OptionTy::Str),
    ("method", OptionTy::Enum(r"Core\Http\Method")),
    ("name", OptionTy::Str),
];
