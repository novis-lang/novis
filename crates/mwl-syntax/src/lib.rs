//! Lexer and parser for MWL.
//!
//! # Layout
//!
//! - [`token`] — the token vocabulary the lexer produces.
//! - [`lexer`] — the dual-mode (inline HTML + code) lexer itself.
//! - [`ast`] — the AST the parser produces: the full type, expression,
//!   statement and declaration grammar M1's plan names.
//! - [`parser`] — the recursive-descent parser covering that whole grammar.
//!   [`parse_file`] is the whole-file entry point (the `mwl-cli` crate's
//!   `mwl ast` uses it); [`Parser`] and [`parse_expression`] are for callers
//!   that want less than a whole file.
//! - [`casing`] — [`check_casing`], the ADR 0029/0030 identifier-casing
//!   check, run directly on a parsed file's declarations; see its module
//!   docs for exactly what is and isn't covered.
//!
//! ```
//! use mwl_diagnostics::{Diagnostics, SourceMap};
//! use mwl_syntax::{tokenize, TokenKind};
//!
//! let mut map = SourceMap::new();
//! let file = map.add("hello.mwl", "<?mwl\necho \"hi\";\n");
//! let mut diags = Diagnostics::new();
//! let tokens = tokenize(map.file(file), &mut diags);
//!
//! assert!(!diags.has_errors());
//! assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
//! ```
//!
//! # Deliberately rejected
//!
//! - **PHP's alternative colon syntax** (`if (...): ... endif;`, and `while`/`for`/`foreach`/`switch`
//!   likewise) is out of scope, not just unbuilt: `endif`/`endfor`/`endforeach`/`endswitch`/`endwhile`/
//!   `enddeclare` are ordinary identifiers, not reserved words, and the parser never looks for a
//!   `:`-delimited body form. Do not add `Keyword::End*` variants or colon-body parsing back.
//!
//! # Known gaps
//!
//! The plan's M1 *Verify* step is a corpus-parse of a real PHP install, which is where these are
//! most likely to bite. None has an ADR-level reason to stay unsupported — they are just not built
//! yet:
//!
//! - **`goto` target labels** (`label:` as its own statement) are unparsed — only `goto ident;`
//!   itself is handled (and rejected, per ADR 0008 § 5).
//! - **Grouped `use`** (`use App\{Foo, Bar};`) and **`use function`/`use const`** are not parsed —
//!   [ADR 0015](../../../docs/adr/0015-no-name-aliasing.md)'s own *Revisiting* note says these don't
//!   exist yet, so this isn't a regression, just not built. Only single `use Path\To\Name;` per
//!   statement is supported.
//! - **Legacy `var $x;`** (PHP 4's property declarator) is not handled — `Keyword::Var` is lexed but
//!   nothing in the parser recognizes it; it falls through to a generic parse error. Vanishingly
//!   rare in modern code.
//! - **A method/const/case name that is itself a reserved keyword spelling** works for methods and
//!   consts but not for enum cases, which require a plain `Ident` — a case literally named e.g.
//!   `Static` would misparse.
//! - **`Core\Static`-style keyword-segment name collisions past the first segment** are still only
//!   spot-checked.

pub mod ast;
mod casing;
mod lexer;
mod parser;
mod token;

pub use casing::check_casing;
pub use lexer::{Lexer, tokenize};
pub use parser::{Parser, parse_expression, parse_file};
pub use token::{Keyword, Token, TokenKind};
