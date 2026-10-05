//! `nvs fmt` — the Novis formatter.
//!
//! One entry, [`format()`]: a [`SourceFile`] goes in, and that file's canonical
//! text comes out or a [`Refusal`] does. `--check`, `--diff` and `--stdin` are
//! I/O modes around that one call, and an editor's formatting request is the
//! same call again — `rule:ide/one-server-two-thin-clients` puts every layout
//! rule here and none of them in a client, because two implementations of them
//! drift the first time one editor's plugin fixes a bug the other's has not.
//!
//! # One style, and no configuration
//!
//! The output is a pure function of the input bytes and of nothing else: no
//! configuration file, no per-project override, and no flag that changes a byte
//! (`rule:tooling/fmt-is-one-canonical-style`). Formatting an
//! already-formatted file changes nothing, which is what makes `--check`
//! well-defined (`rule:tooling/fmt-is-idempotent`). The author's own line
//! breaks choose the layout: a list with a break at its own level has one item
//! per line, and nothing here decides by itself where an expression breaks, so
//! there is no width limit anywhere in it (`rule:tooling/fmt-never-reflows`).
//!
//! # The tree it reads
//!
//! [`nvs_syntax::parse`], and nothing else. The strict
//! [`nvs_syntax::parse_file`] keeps no trivia, so a walk over it would delete
//! every comment in the file; the lossless parse hands back the same statements
//! with every comment and whitespace run beside them, one grammar and one tree
//! (`rule:ide/one-grammar-one-tree`). What the printer rests on is
//! `rule:ide/tokens-plus-trivia-reproduce-the-file`, which
//! `crates/nvs-syntax/tests/lossless.rs` holds over the whole corpus.
//!
//! # A file the parse could not place is refused
//!
//! A parse that reports an error answers a [`Refusal`] carrying that parse's
//! diagnostics, and no text at all. The safe direction is the whole reason: an
//! editor saving a half-written file gets it back unchanged rather than
//! rearranged around a bracket its author has not typed yet. The parse is
//! [`format()`]'s own, so a caller walking a directory hands each file in
//! separately and one file's errors can never refuse the next one.
//!
//! The one error that refuses nothing is the one this formatter itself
//! rewrites: a mis-cased reserved spelling
//! (`rule:tooling/fmt-normalizes-only-reserved-spellings`). Nothing about such
//! a file is half written — the lexer read the tag or the duration it
//! was handed, every span behind it is exact, and the diagnostic names the
//! bytes to lower-case — so [`crate::tokens`] writes the spelling and the file
//! is formatted. A rule naming a rewrite that no file carrying one ever reaches
//! would be a rule with nothing behind it.
//!
//! # What it spends
//!
//! One file's tree and one output string at a time, both dropped before the
//! next file is read. This is a command-line tool and an editor request:
//! nothing here runs on the request path or inside the runtime.
//!
//! # Known gaps
//!
//! Each gap is a record, and `bun nv gaps --module crates/nvs-fmt/src/lib.rs` lists them.

use std::fmt;

use nvs_diagnostics::{Diagnostics, SourceFile};

mod brace;
mod chain;
mod imports;
mod indent;
mod list;
mod modifiers;
mod print;
mod space;
mod tokens;

/// A file `nvs fmt` will not rewrite, because its parse reported an error
/// these rules do not themselves rewrite.
///
/// Carries the file's name and everything its parse reported, so a caller can
/// name it and render the diagnostics through the renderer it already has. It
/// deliberately carries no text: "refused" is then impossible to mistake for
/// "formatted, to the same bytes".
#[derive(Debug)]
pub struct Refusal {
    name: String,
    diagnostics: Diagnostics,
}

impl Refusal {
    /// The refused file's display name, as a diagnostic would spell it.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Everything the parse reported, in the order it reported it.
    #[must_use]
    pub const fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// The same diagnostics, owned, for a caller whose renderer sorts them.
    ///
    /// [`Diagnostics::sort_by_position`] needs them by value, and a refusal has
    /// nothing left to say once its diagnostics have been rendered.
    #[must_use]
    pub fn into_diagnostics(self) -> Diagnostics {
        self.diagnostics
    }
}

impl fmt::Display for Refusal {
    /// One line for a terminal. The diagnostics themselves are left to the
    /// caller's renderer rather than printed twice in two shapes.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: not formatted, because it does not parse ({} error(s))",
            self.name,
            self.diagnostics.error_count()
        )
    }
}

impl std::error::Error for Refusal {}

/// Rewrites `file` into its one canonical layout and answers the whole text.
///
/// # Errors
///
/// A file whose parse reports an error the token rules do not themselves
/// rewrite is refused: the answer is a [`Refusal`] naming it and carrying the
/// diagnostics, and no text is produced.
pub fn format(file: &SourceFile) -> Result<String, Refusal> {
    let mut diagnostics = Diagnostics::new();
    let parsed = nvs_syntax::parse(file, &mut diagnostics);
    if diagnostics
        .iter()
        .any(|d| d.is_error() && !tokens::repairs(d))
    {
        return Err(Refusal {
            name: file.name().to_owned(),
            diagnostics,
        });
    }
    Ok(print::print(file, &parsed, &diagnostics))
}
