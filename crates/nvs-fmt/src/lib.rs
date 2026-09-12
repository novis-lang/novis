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
//! breaks are kept: this normalizes what surrounds an expression and never
//! decides where one breaks, so there is no width limit anywhere in it
//! (`rule:tooling/fmt-never-reflows`).
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
//! # A file that does not parse is refused
//!
//! A parse that reports an error answers a [`Refusal`] carrying that parse's
//! diagnostics, and no text at all. The safe direction is the whole reason: an
//! editor saving a half-written file gets it back unchanged rather than
//! rearranged around a bracket its author has not typed yet. The parse is
//! [`format()`]'s own, so a caller walking a directory hands each file in
//! separately and one file's errors can never refuse the next one.
//!
//! # What it spends
//!
//! One file's tree and one output string at a time, both dropped before the
//! next file is read. This is a command-line tool and an editor request:
//! nothing here runs on the request path or inside the runtime.
//!
//! # Known gaps
//!
//! 1. **Indentation and modifier order are the layout rules written into the
//!    printer.** The run walk `rule:ide/tokens-plus-trivia-reproduce-the-file`
//!    names is the frame every rule under `docs/rules/tooling/fmt-*` hangs off,
//!    and what lands in it so far is four spaces per enclosing body
//!    (`indent.rs`) and one canonical order for a declaration's modifiers
//!    (`modifiers.rs`). Brace placement, PER's blank lines, the quote and
//!    trailing-comma rules, the `use` block's order and the constructs PER
//!    never saw are still the author's, so a file that disagrees with one of
//!    them comes back disagreeing with it.
//!    — owner: M10
//! 2. **A line the tree does not place keeps the author's own indentation.**
//!    `indent.rs`'s own doc says which ones those are: a `switch`, a `match`
//!    arm list, a template region, and every continuation line inside an
//!    expression. The first three are rules that have not landed; the last is
//!    `rule:tooling/fmt-never-reflows` and stays the author's for good.
//!    — owner: M10

use std::fmt;

use nvs_diagnostics::{Diagnostics, SourceFile};

mod indent;
mod modifiers;
mod print;

/// A file `nvs fmt` will not rewrite, because it does not parse.
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
/// A file whose parse reports an error is refused: the answer is a [`Refusal`]
/// naming it and carrying the diagnostics, and no text is produced.
pub fn format(file: &SourceFile) -> Result<String, Refusal> {
    let mut diagnostics = Diagnostics::new();
    let parsed = nvs_syntax::parse(file, &mut diagnostics);
    if diagnostics.has_errors() {
        return Err(Refusal {
            name: file.name().to_owned(),
            diagnostics,
        });
    }
    Ok(print::print(file, &parsed))
}
