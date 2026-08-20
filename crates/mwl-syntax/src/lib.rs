//! Lexer and parser for MWL.
//!
//! # Layout
//!
//! - [`token`] — the token vocabulary the lexer produces.
//! - [`lexer`] — the dual-mode (inline HTML + code) lexer itself.
//!
//! The parser (AST + spans, M1's other half) is not written yet; this crate
//! currently covers only tokenization.
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

mod lexer;
mod token;

pub use lexer::{Lexer, tokenize};
pub use token::{Keyword, Token, TokenKind};
