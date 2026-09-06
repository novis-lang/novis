//! Lexer and parser for Novis.
//!
//! # Layout
//!
//! - [`token`] — the token vocabulary the lexer produces.
//! - [`lexer`] — the dual-mode (inline HTML + code) lexer itself.
//! - [`ast`] — the AST the parser produces: the full type, expression,
//!   statement and declaration grammar M1's plan names.
//! - [`parser`] — the recursive-descent parser covering that whole grammar.
//!   [`parse_file`] is the whole-file entry point (the `nvs-cli` crate's
//!   `nvs ast` uses it); [`Parser`] and [`parse_expression`] are for callers
//!   that want less than a whole file.
//! - [`casing`] — [`check_declarations`], the two rules a parsed file's
//!   declarations answer on their own: `rule:core-api/identifier-casing`/0030's identifier casing and
//!   `rule:core-api/written-visibility`'s required member visibility. See its module docs for exactly
//!   what is and isn't covered.
//! - ADR 0087's unterminated-directional-scope predicate is
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
//!
//! # Known gaps
//!
//! The plan's M1 *Verify* step is a corpus-parse of a real PHP install, which is where these are
//! most likely to bite. None has an ADR-level reason to stay unsupported — they are just not built
//! yet:
//!
//! - **`goto` target labels** (`label:` as its own statement) are unparsed — only `goto ident;`
//!   itself is handled (and rejected, per `rule:statements/no-function-static-and-no-global`). Interacts with
//!   `rule:types/object-literal`'s own block/object-literal
//!   disambiguation: a block whose first statement would have been a label (`{ done: ... }`) matches
//!   the same one-token-past-`{` lookahead an attempted object literal does, so it is now diagnosed
//!   as "needs parentheses" instead of whatever the (already broken, since labels don't parse) prior
//!   behavior was — not a regression on real code, since no Novis/PHP program relies on an unparsed
//!   construct, but worth knowing if label support is ever added.
//! - **A local variable declaration typed with a bare inline shape type** (`{x: int} $point;`) is not
//!   parsed — statement-initial `{` already commits to a block (`rule:types/shape-type`), and unlike the object-literal collision that ADR names and this parser resolves, teaching
//!   a *type*-prefix apart from a block would need lookahead past a matched, possibly-nested `{...}`
//!   all the way to a following `$name` — not attempted this session. Every other declaration slot
//!   (parameter, return type, property, class constant, `foreach` binding) supports a bare shape type
//!   fine; the workaround for a local is the same one the ADR's own example uses: `type Point = {x:
//!   int}; Point $point;`.
//! - **Grouped `use`** (`use App\{Foo, Bar};`) and **`use function`/`use const`** are not parsed —
//!   `rule:statements/nothing-gets-a-second-name`'s own *Revisiting* note says these don't
//!   exist yet, so this isn't a regression, just not built. Only single `use Path\To\Name;` per
//!   statement is supported.
//! - **`var` inside a class body** (PHP 4's property declarator) is not handled — only the statement
//!   position now recognizes `Keyword::Var`, as `rule:types/var-inference`'s
//!   inferred local declaration; the property-declarator spelling still falls through to a generic
//!   parse error. Vanishingly rare in modern code.
//! - **A method/const/case name that is itself a reserved keyword spelling** works for methods and
//!   consts but not for enum cases, which require a plain `Ident` — a case literally named e.g.
//!   `Static` would misparse.
//! - **`Core\Static`-style keyword-segment name collisions past the first segment** are still only
//!   spot-checked.

pub mod ast;
mod casing;
pub mod duration;
mod lexer;
mod parser;
mod token;
pub mod walk;

pub use casing::check_declarations;
pub use lexer::{Lexer, tokenize};
pub use parser::{Parser, parse_expression, parse_file};
pub use token::{Keyword, Token, TokenKind};
