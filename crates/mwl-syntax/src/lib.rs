//! Lexer and parser for MWL.
//!
//! # Layout
//!
//! - [`token`] — the token vocabulary the lexer produces.
//! - [`lexer`] — the dual-mode (inline HTML + code) lexer itself.
//! - [`ast`] — the AST the parser produces: types and expressions so far, see
//!   its module docs for what is deliberately not there yet.
//! - [`parser`] — the recursive-descent parser: the full type and expression
//!   grammar. Statement and declaration grammar (control flow, classes,
//!   interfaces, traits, enums) are the parser's next two chunks.
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

pub mod ast;
mod lexer;
mod parser;
mod token;

pub use lexer::{Lexer, tokenize};
pub use parser::{Parser, parse_expression};
pub use token::{Keyword, Token, TokenKind};
