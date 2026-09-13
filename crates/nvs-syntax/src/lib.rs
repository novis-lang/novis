//! Lexer and parser for Novis.
//!
//! # Layout
//!
//! - [`token`] — the token vocabulary the lexer produces, and the [`Trivia`]
//!   runs it skips between two tokens.
//! - [`lexer`] — the dual-mode (inline HTML + code) lexer itself.
//! - [`ast`] — the AST the parser produces: the full type, expression,
//!   statement and declaration grammar M1's plan names.
//! - [`parser`] — the recursive-descent parser covering that whole grammar.
//!   [`parse_file`] is the whole-file entry point every compile path calls (the
//!   `nvs-cli` crate's `nvs ast` uses it); [`parse`] is the same parse plus what
//!   a caller reproducing the file rather than compiling it needs and a compile
//!   path never asks for ([`Parsed`]): the trivia between the statements, and
//!   the span each modifier was written at, which is how `nvs fmt` puts a
//!   modifier list in one canonical order without a second walk over the
//!   grammar (`rule:ide/one-grammar-one-tree`);
//!   [`Parser`] and [`parse_expression`] are for callers that want less than a
//!   whole file. A `///` run is not part of that side channel — both whole-file
//!   entry points attach one to the declaration below it and refuse one that
//!   documents nothing
//!   (`rule:tooling/doc-comment-attaches-to-the-next-declaration`), because the
//!   language reads documentation rather than skipping it.
//! - [`casing`] — [`check_declarations`], the two rules a parsed file's
//!   declarations answer on their own: `rule:core-api/identifier-casing`/0030's identifier casing and
//!   `rule:core-api/written-visibility`'s required member visibility. See its module docs for exactly
//!   what is and isn't covered.
//! - `rule:security/bidi-predicate`'s unterminated-directional-scope predicate is
//!   [`nvs_render::bidi`], not this crate's: the lexer makes it a hard error
//!   and reads it from below. It moved there when `rule:errors/log-write`'s tier-4 floor
//!   became a dependent of `nvs-render`, which `rule:errors/diagnostic-record` requires be the
//!   leaf — that crate's own § *Where this sits* is the home of why.
//! - [`duration`] — `rule:types/duration-literal`'s duration grammar, the one place `30s` is
//!   defined. Public because it is shared: `nvs-stdlib`'s
//!   `Core\Time\Duration::parse` and (at M6) `nvs.toml`'s reader both call in,
//!   which is what stops the three from drifting.
//!
//! ```
//! use nvs_diagnostics::{Diagnostics, SourceMap};
//! use nvs_syntax::{tokenize, TokenKind};
//!
//! let mut map = SourceMap::new();
//! let file = map.add("hello.nvs", "<?nvs\necho \"hi\";\n");
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
//! - **PHP 8.5's pipe operator** (`$x |> strlen(...)`) does not parse and never has — `|>` is not a
//!   token, and there is no plan to add one. It is pure call-chain sugar (`$x |> f(...) |> g(...)` is
//!   just `g(f($x))`), so it adds no expressiveness a nested call or a local variable doesn't already
//!   give, while costing a new operator with its own precedence tier and a special-cased RHS shape
//!   (reusing the `...` first-class-callable placeholder from `rule:types/callable-is-a-closure`). It also undercuts its own
//!   usual justification here: `rule:classes/no-free-functions-or-constants` makes every function a method, so idiomatic Novis code already
//!   reaches for `->` chaining instead of PHP's global-function nesting, which is the pain `|>` exists
//!   to solve in vanilla PHP. Do not add a `Pipe`/`|>` token or an `ExprKind::Pipe` node.
//! - **A grouped import** (`use App\{Foo, Bar};`) is refused rather than unbuilt — the parser eats
//!   the `\{...}` and reports `E_IMPORT_GROUP_UNSUPPORTED`, whose help names the one supported
//!   spelling: one `use` statement per imported name, each ending in the short name it introduces.
//!   That is the shape of the rename refusal beside it
//!   (`rule:statements/nothing-gets-a-second-name`) and holds for the same reason — a reader learns
//!   every short name a file introduces from the statement that introduces it.
//! - **A `goto` label** (`done:` as its own statement) is not parsed, and gets no refusal of its own.
//!   `goto` itself is refused (`E0203`, in `parser/stmt.rs`'s `parse_goto`), that is the first error
//!   an author reads, and a label has nothing else to be for — so the generic errors behind it are
//!   left as they are. `use function` and `use const` are the same shape: `function` and `const`
//!   stay ordinary name segments, because `rule:classes/no-free-functions-or-constants` leaves
//!   neither spelling anything to import.
//!
//! # Known gaps
//!
//! Neither has a decision record's reason to stay unsupported — each is just not built yet:
//!
//! - **A local variable declaration typed with a bare inline shape type** (`{x: int} $point;`) is not
//!   parsed — statement-initial `{` already commits to a block (`rule:types/shape-type`), and unlike
//!   the object-literal collision `rule:types/object-literal` names and this parser resolves, teaching
//!   a *type*-prefix apart from a block would need lookahead past a matched, possibly-nested `{...}`
//!   all the way to a following `$name`. Every other declaration slot
//!   (parameter, return type, property, class constant, `foreach` binding) supports a bare shape type
//!   fine; the workaround for a local is the one that rule's own example uses: `type Point = {x:
//!   int}; Point $point;`.
//!   Decided: Keep the alias form and give a targeted error — No parser cost, a clear message pointing
//!   at `type Point = {...}; Point $p;`, and one slot is narrower than the rest.
//!   — owner: unowned-closures
//! - **`use function` and `use const`** are not parsed — `function` and `const` are ordinary name
//!   segments, so `use function Foo\bar;` reads `function` as the imported name and then fails on
//!   `Foo` with a generic parse error. What a real corpus needs here is the *refusal* rather than the
//!   feature: `rule:classes/no-free-functions-or-constants` leaves neither spelling anything to
//!   import.
//!   — owner: M1
//! - **`Core\Static`-style keyword-segment name collisions past the first segment** are still only
//!   spot-checked. Every segment takes a keyword spelling by construction — `Parser::is_name_segment`
//!   is `Ident | Keyword(_)` — so what is thin here is the coverage, not the grammar.
//!   — owner: M1

pub mod ast;
mod casing;
pub mod duration;
pub mod index;
mod lexer;
mod parser;
mod token;
pub mod walk;

pub use casing::check_declarations;
pub use index::{IndexNode, NodePath, SyntaxIndex};
pub use lexer::{Lexer, tokenize};
pub use parser::{DOC_MARKER, Parsed, Parser, parse, parse_expression, parse_file};
pub use token::{Keyword, Token, TokenKind, Trivia, TriviaKind};
