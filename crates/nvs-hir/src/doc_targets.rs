//! What each doc comment tag that names something resolved to: a `@see`
//! target, an `@example` path and an `@throws` class.
//!
//! [`crate::members`] resolves every tag once, while it checks it
//! (`rule:tooling/doc-comment-tags-are-closed-and-checked`). The language
//! server's links, go-to-definition, hover, semantic tokens, references and
//! renames read this record and never parse a tag again, which is the
//! discipline `nvs_lsp::links` states for every link: an edge the check
//! resolved is read, not derived a second time.
//!
//! The record is keyed by the tag argument's span, which carries its file.
//! A tag that did not resolve has no entry: its diagnostic says why.
//!
//! # What it costs
//!
//! Nothing for a check that does not record: [`DocTargets::off`] stores
//! nothing. It records on the same road as [`crate::uses`], only when
//! [`crate::requires::resolve_program`] gets a parse cache, which is the
//! language server's path. A recording check keeps one entry per resolved tag
//! for as long as the check's output is kept.

use std::path::PathBuf;

use nvs_diagnostics::{SourceId, Span};
use rustc_hash::FxHashMap;

use crate::QName;
pub use crate::members::MemberKind;

/// What one tag's argument resolved to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolved {
    /// A class, interface or enum: a `@see Class` target, the class half of
    /// a `@see Class::member` and every `@throws` name.
    Class(QName),
    /// A member found on `class` or on a class it inherits from. `class` is
    /// the class the target named, and `owner` the one that declares it.
    Member {
        /// The class the target named, after `self`, `static` and `parent`.
        class: QName,
        /// The class that declares the member.
        owner: QName,
        /// The member's name, without a `$` sigil or `()`.
        name: String,
        /// Which kind of member it is. An enum case is a [`MemberKind::Const`].
        kind: MemberKind,
    },
    /// A file: a `@see` file target or an `@example` path.
    File {
        /// The file, absolute and canonical.
        path: PathBuf,
        /// The 0-based line of the heading the anchor named, if one was
        /// written.
        heading: Option<u32>,
    },
}

/// One resolved tag argument.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocTarget {
    /// The argument's span: the whole written target, anchor included.
    pub span: Span,
    /// The span of the class half of a `Class::member` target, when one was
    /// written and resolved. A rename of the class rewrites this span only.
    pub class_span: Option<Span>,
    /// What it resolved to.
    pub resolved: Resolved,
}

/// Every resolved tag argument of a check, by file.
#[derive(Clone, Debug, Default)]
pub struct DocTargets {
    on: bool,
    files: FxHashMap<SourceId, Vec<DocTarget>>,
}

impl DocTargets {
    /// A record that stores nothing.
    #[must_use]
    pub fn off() -> Self {
        Self::default()
    }

    /// An empty record that stores.
    #[must_use]
    pub fn recording() -> Self {
        Self {
            on: true,
            ..Self::default()
        }
    }

    /// Whether this record stores.
    #[must_use]
    pub const fn is_on(&self) -> bool {
        self.on
    }

    /// Stores one resolved argument, if this record stores.
    pub fn push(&mut self, target: DocTarget) {
        if self.on {
            self.files.entry(target.span.file).or_default().push(target);
        }
    }

    /// Every resolved argument written in `file`, in the order the check
    /// reached them.
    #[must_use]
    pub fn in_file(&self, file: SourceId) -> &[DocTarget] {
        self.files.get(&file).map_or(&[], Vec::as_slice)
    }

    /// The resolved argument whose span contains `offset` in `file`.
    #[must_use]
    pub fn at(&self, file: SourceId, offset: u32) -> Option<&DocTarget> {
        self.in_file(file)
            .iter()
            .find(|target| target.span.start <= offset && offset <= target.span.end)
    }

    /// Every resolved argument of every file.
    pub fn iter(&self) -> impl Iterator<Item = &DocTarget> {
        self.files.values().flatten()
    }
}
