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
//! - [`string_lit`] — what a written string literal *denotes*: the escape
//!   grammar cooked to bytes, and a heredoc's flexible-indentation strip. The
//!   lexer decides which backslash sequences exist; this decides what they
//!   produce, which needs the whole literal assembled and so cannot happen at
//!   lex time. Public for the same reason [`duration`] is, and the reason is
//!   stronger: `nvs-hir` reads a `require` path out of a literal, `nvs-types`
//!   diagnoses and interns one, `nvs-ir` lowers one, and all three must agree
//!   byte for byte — see its own module docs.
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
//! - **A pipeline *node*.** `|>` is a token (`TokenKind::PipeGreater`) and parses; what it is not is
//!   PHP 8.5's operator, which applies a callable where Novis's substitutes the hole `$_` at compile
//!   time (`rule:expressions/pipeline-substitution`). `Parser::parse_pipe` parks the left side in
//!   `Parser::pipe_hole` and `parse_hole` hands it to the `$_` on the right, so `$x |> Str::trim($_)`
//!   yields exactly the tree `Str::trim($x)` does and every later stage sees the substitution rather
//!   than the operator. Do not add an `ExprKind::Pipe` node or a node for the hole: a right side with
//!   no `$_`, one with a second, and a `$_` outside a pipeline are the three refusals
//!   `rule:expressions/pipeline-hole-once` names — `E_PIPELINE_RIGHT_SIDE_HAS_NO_HOLE`,
//!   `E_PIPELINE_RIGHT_SIDE_REPEATS_THE_HOLE` and `E_HOLE_OUTSIDE_A_PIPELINE` — not shapes the tree
//!   carries.
//! - **A grouped import** (`use App\{Foo, Bar};`) is refused rather than unbuilt — the parser eats
//!   the `\{...}` and reports `E_IMPORT_GROUP_UNSUPPORTED`, whose help names the one supported
//!   spelling: one `use` statement per imported name, each ending in the short name it introduces.
//!   That is the shape of the rename refusal beside it
//!   (`rule:statements/nothing-gets-a-second-name`) and holds for the same reason — a reader learns
//!   every short name a file introduces from the statement that introduces it.
//! - **A `goto` label** (`done:` as its own statement) is not parsed, and gets no refusal of its own.
//!   `goto` itself is refused (`E0203`, in `parser/stmt.rs`'s `parse_goto`), that is the first error
//!   an author reads, and a label has nothing else to be for — so the generic errors behind it are
//!   left as they are.
//! - **`use function` and `use const`** are refused rather than parsed — `E_IMPORT_OF_FUNCTION_OR_CONST_UNSUPPORTED`
//!   on the keyword, whose help names the class member each spelling's target is written as, since
//!   `rule:classes/no-free-functions-or-constants` leaves neither anything to import. The keyword is
//!   eaten and the path behind it parses as the ordinary import it looks like. `function` and `const`
//!   remain ordinary name segments everywhere else, so `use function\Foo;` is a namespace whose first
//!   segment is spelled `function` and is not this refusal.
//! - **A local declaration typed with a bare inline shape** (`{x: int} $point;`) is refused rather
//!   than parsed — `E_SHAPE_TYPED_LOCAL_NEEDS_AN_ALIAS`, whose help names the `type` alias form
//!   `rule:types/shape-type`'s own example uses. A statement-initial `{` opens a block before it is
//!   anything else, so this is the one declaration slot a bare shape cannot fill, while every other
//!   one (parameter, return type, property, class constant, `foreach` binding) takes it. The tell is
//!   `parser/stmt.rs`'s `Parser::at_shape_typed_local`: a braced run that opens like a field list
//!   *and* a variable after the matched `}`. That is what leaves a block a variable happens to follow
//!   (`{ echo 1; } $x = 1;`) with its ordinary parse, and a discarded object literal with
//!   `rule:types/object-literal`'s own refusal.

pub mod ast;
mod casing;
pub mod duration;
pub mod index;
mod lexer;
mod parser;
pub mod string_lit;
mod token;
pub mod visit;
pub mod walk;

pub use casing::check_declarations;
pub use index::{IndexNode, NodePath, SyntaxIndex};
pub use lexer::{Lexer, tokenize};
pub use parser::{DOC_MARKER, Parsed, Parser, parse, parse_expression, parse_file};
pub use token::{Keyword, Token, TokenKind, Trivia, TriviaKind};
