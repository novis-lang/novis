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
//! a file is half written — the lexer read the tag or the duration literal it
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
//! 1. **The rules written into the printer are the ones its own modules
//!    name.** The run walk `rule:ide/tokens-plus-trivia-reproduce-the-file`
//!    names is the frame every rule under `docs/rules/tooling/fmt-*` hangs off,
//!    and what lands in it is four spaces per enclosing body (`indent.rs`), one
//!    canonical order for a declaration's modifiers (`modifiers.rs`), the line
//!    each opening brace sits on (`brace.rs`), the order a run of imports goes
//!    out in (`imports.rs`), the run a qualified type, a one-line object
//!    literal and each arm of a multi-line `match` are written with
//!    (`space.rs`), and the quote a plain string
//!    literal is delimited
//!    by together with the comma a multi-line list ends its last element on and
//!    the one case a reserved spelling has (`tokens.rs`). PER's blank lines are
//!    still the author's, so a file that disagrees with them comes back
//!    disagreeing with them.
//!    — owner: M10
//! 2. **A line the tree does not place keeps the author's own indentation.**
//!    `indent.rs`'s own doc says which ones those are: a `switch`, a template
//!    region's own text, every continuation line inside an expression, and a
//!    line opening with a comment, which starts no node for the index to answer
//!    at — except inside a `match`, where it opens the arm it precedes and is
//!    placed with it. The first two are rules that have not landed — the `?>`
//!    that opens a template region is code and is placed, the text after it is
//!    not — a comment line is `rule:tooling/fmt-base-style-is-per` reaching a
//!    line the index has nothing to say about, and the last is
//!    `rule:tooling/fmt-never-reflows` and stays the author's for good.
//!    — owner: M10
//! 3. **A closing brace is moved onto a line of its own only where it already
//!    opens one.** `brace.rs` decides the run before an *opening* brace and the
//!    one before an `elseif`, `else`, `catch` or `finally`; a `}` sharing a
//!    line with the statement before it stays there, which is the half of
//!    `rule:tooling/fmt-base-style-is-per`'s brace paragraph that needs
//!    one-statement-per-line to land with it.
//!    — owner: M10
//! 4. **A literal written inside an attribute keeps the quotes it was written
//!    with.** `tokens.rs` respells a literal the index has a node for, and an
//!    attribute's argument list is written outside every node
//!    `crates/nvs-syntax/src/walk.rs` builds — the same absence `brace.rs`'s
//!    own doc names for an attribute's object literal. So
//!    `#[Core\Command(name: "greet")]` comes back double-quoted where the same
//!    literal in an argument list does not, and what closes it is a node for
//!    the attribute in that walk rather than a second scan here.
//!    — owner: M10
//! 5. **A comma-separated list whose closing delimiter is not its node's last
//!    byte keeps the comma its author wrote.** `tokens.rs` tells a list's
//!    closer from a block's by the node that ends there, and three lists end
//!    inside a node that runs on past them: a parameter list inside the
//!    `Function` or `Method` whose body follows it, an enum-case list inside
//!    the `EnumDecl`, and a shape type's fields inside a declaration whose
//!    type `crates/nvs-syntax/src/walk.rs` gives no node at all. So
//!    `rule:tooling/fmt-trailing-commas` reaches call arguments and array,
//!    object and `match` arm lists today, and what closes the rest is a
//!    boundary for each in that walk rather than a bracket-matching scan here,
//!    which would have to tell a `)` in an attribute's argument list from the
//!    one that ends the parameters — gap 4's absence again. A shape type's
//!    braces are outside `space.rs`'s one-space rule for the same reason they
//!    are outside this one: `{a: string}` is a node to nobody, so the `{` that
//!    opens it cannot be told from the `{` that opens a block.
//!    — owner: M10
//! 6. **A comment written between two imports holds the block it is in
//!    unsorted on either side of it.** A comment belongs to the declaration it
//!    was written above, and `imports.rs` moves a path without moving anything
//!    around it, so sorting across one would leave it standing over an import
//!    it does not describe. The safe direction is the one taken: the run ends
//!    at the comment and each half is sorted alone, so
//!    `rule:tooling/fmt-sorts-the-use-block` is satisfied for an uncommented
//!    block and under-applied for a commented one. What closes it is the
//!    comment moving with the declaration below it, which is a second span per
//!    element rather than a second scan.
//!    — owner: M10

use std::fmt;

use nvs_diagnostics::{Diagnostics, SourceFile};

mod brace;
mod imports;
mod indent;
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
