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
//! # The modules that write a rule
//!
//! The run walk in [`crate::print`] is the frame every rule under
//! `docs/rules/tooling/fmt-*` hangs off, and each rule lands in it from one
//! module:
//!
//! - [`crate::indent`]: four spaces per enclosing body.
//! - [`crate::statement`]: one statement per line, and a closing brace on a
//!   line of its own.
//! - [`crate::brace`]: the line each opening brace sits on, `elseif`, `else`,
//!   `catch` and `finally` on the closing brace's line, and `elseif` as one
//!   word.
//! - [`crate::blank`]: the blank line after `namespace`, after the `use` block
//!   and between two members that each have a body.
//! - [`crate::modifiers`]: one canonical order for a declaration's modifiers.
//! - [`crate::imports`]: the order a run of imports goes out in.
//! - [`crate::list`]: a list on one line or one item per line.
//! - [`crate::chain`]: a call chain or an operator chain on one line or one
//!   link per line.
//! - [`crate::space`]: the run a qualified type, a one-line anonymous object
//!   and each arm of a multi-line `match` are written with.
//! - [`crate::gap`]: nothing or one space between two tokens on one line,
//!   from what the two tokens are.
//! - [`crate::tokens`]: the quote a plain string literal is delimited by, the
//!   comma a multi-line list ends its last item on, and the one case a reserved
//!   spelling has.
//!
//! [`crate::split_join`] writes no rule of its own: its edits are laid out the
//! way [`crate::list`] and [`crate::chain`] lay them out.

use std::fmt;

use nvs_diagnostics::{Diagnostics, SourceFile, SourceMap};

mod blank;
mod brace;
mod chain;
mod gap;
mod imports;
mod indent;
mod list;
mod modifiers;
mod print;
mod space;
mod split_join;
mod statement;
mod tokens;

pub use split_join::{Relayout, Toggle};

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
    let Some(sorted) = imports::sorted(&parsed, file.text()) else {
        return Ok(print::print(file, &parsed, &diagnostics));
    };
    // The imports moved whole, so the text parses as the file did, and every
    // other rule places the moved lines from this second parse.
    let mut map = SourceMap::new();
    let id = map.add(file.name(), sorted);
    let file = map.file(id);
    let mut diagnostics = Diagnostics::new();
    let parsed = nvs_syntax::parse(file, &mut diagnostics);
    Ok(print::print(file, &parsed, &diagnostics))
}

/// The text of `file` with every run of `use` declarations in the order
/// `rule:tooling/fmt-sorts-the-use-block` gives, and nothing else changed.
///
/// `Ok(None)` where every run is already in order. Organize imports in the
/// editor calls this, so a format after it moves no import.
///
/// # Errors
///
/// A file that does not parse is refused, on [`format()`]'s terms.
pub fn sorted_imports(file: &SourceFile) -> Result<Option<String>, Refusal> {
    let mut diagnostics = Diagnostics::new();
    let parsed = nvs_syntax::parse(file, &mut diagnostics);
    if diagnostics
        .iter()
        .any(nvs_diagnostics::Diagnostic::is_error)
    {
        return Err(Refusal {
            name: file.name().to_owned(),
            diagnostics,
        });
    }
    Ok(imports::sorted(&parsed, file.text()))
}

/// The edit that puts the list around byte `offset` of `file` on separate
/// lines, or joins it onto one, written as `nvs fmt` lays it out
/// (`rule:ide/a-list-splits-onto-lines-and-joins-onto-one`).
///
/// [`None`] where no list is around `offset`, where the file does not parse,
/// and where the list is broken and a line comment sits at its own level.
#[must_use]
pub fn relayout(file: &SourceFile, offset: usize) -> Option<Relayout> {
    split_join::at(file, offset)
}
