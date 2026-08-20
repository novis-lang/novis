//! Source positions, source maps and diagnostics for MWL.
//!
//! Every later stage of the compiler depends on this crate and nothing else
//! depends on those stages, which keeps the dependency graph a line rather than
//! a web.
//!
//! ```
//! use mwl_diagnostics::{code, Diagnostic, Diagnostics, Renderer, SourceMap, Span};
//!
//! let mut map = SourceMap::new();
//! let file = map.add("greet.mwl", "<?mwl\necho $undefined;\n");
//!
//! let mut sink = Diagnostics::new();
//! sink.report(
//!     Diagnostic::error(code::E_UNDEFINED_VARIABLE, "undefined variable `$undefined`")
//!         .with_primary(Span::new(file, 11, 21), "not defined in this scope")
//!         .with_help("declare it before use, or check the spelling"),
//! );
//!
//! assert!(sink.has_errors());
//!
//! let mut out = Vec::new();
//! Renderer::new().render_all(sink.iter(), &map, &mut out).unwrap();
//! let text = String::from_utf8(out).unwrap();
//! assert!(text.contains("error[E0301]"));
//! assert!(text.contains("aborting due to 1 error"));
//! ```

mod diagnostic;
mod render;
mod source;
mod span;

pub use diagnostic::{Code, Diagnostic, Diagnostics, Label, LabelStyle, Severity, Suggestion};
pub use render::Renderer;
pub use source::{MAX_SOURCE_LEN, SourceFile, SourceMap};
pub use span::{BytePos, SourceId, Span, Spanned};

/// Stable diagnostic codes.
///
/// A code is a promise: once released, its meaning never changes. Retire a code
/// rather than repurposing it, because users write suppressions against them and
/// tooling matches on them.
///
/// Ranges are allocated by compiler stage so that a code alone tells you where
/// it came from:
///
/// | range | stage |
/// |---|---|
/// | `E00xx` | lexer |
/// | `E01xx` | parser |
/// | `E02xx` | rejected PHP constructs (the "pragmatic superset" exclusions) |
/// | `E03xx` | name resolution |
/// | `E04xx` | types |
/// | `E05xx` | IR and codegen |
/// | `E06xx` | configuration and capabilities |
/// | `E09xx` | internal compiler errors |
/// | `W1xxx` | warnings |
pub mod code {
    use crate::diagnostic::Code;

    // --- E00xx lexer -------------------------------------------------------
    /// A character that cannot begin any token.
    pub const E_UNEXPECTED_CHAR: Code = Code::new("E0001");
    /// A string, comment or heredoc that runs to end of file.
    pub const E_UNTERMINATED: Code = Code::new("E0002");
    /// A numeric literal the lexer cannot interpret.
    pub const E_INVALID_NUMBER: Code = Code::new("E0003");
    /// A backslash escape that is not defined.
    pub const E_INVALID_ESCAPE: Code = Code::new("E0004");
    /// A heredoc whose closing identifier is missing or misindented.
    pub const E_BAD_HEREDOC: Code = Code::new("E0005");
    /// Input that is not valid UTF-8.
    pub const E_INVALID_UTF8: Code = Code::new("E0006");

    // --- E01xx parser ------------------------------------------------------
    /// A specific token was required here.
    pub const E_EXPECTED_TOKEN: Code = Code::new("E0101");
    /// An expression was required here.
    pub const E_EXPECTED_EXPR: Code = Code::new("E0102");
    /// A statement was required here.
    pub const E_EXPECTED_STMT: Code = Code::new("E0103");
    /// A bracket, brace or paren was opened and never closed.
    pub const E_UNCLOSED_DELIMITER: Code = Code::new("E0104");
    /// Assignment to something that cannot be assigned to.
    pub const E_INVALID_ASSIGN_TARGET: Code = Code::new("E0105");
    /// A modifier that is not allowed in this position.
    pub const E_BAD_MODIFIER: Code = Code::new("E0106");
    /// A parameter list that is malformed — duplicate names, or a required
    /// parameter after an optional one.
    pub const E_BAD_PARAM_LIST: Code = Code::new("E0107");

    // --- E02xx rejected PHP constructs -------------------------------------
    // MWL accepts PHP 8.5 syntax as a *pragmatic* superset. These constructs
    // parse — so the diagnostic can be precise and suggest a replacement —
    // and are then rejected. See docs/spec.
    /// `eval()`: MWL compiles ahead of execution.
    pub const E_EVAL_UNSUPPORTED: Code = Code::new("E0201");
    /// `$$name` and `${$name}`: defeats name resolution and type inference.
    pub const E_VARIABLE_VARIABLE: Code = Code::new("E0202");
    /// `goto`: makes the control-flow graph unstructured.
    pub const E_GOTO_UNSUPPORTED: Code = Code::new("E0203");
    /// `global $x`: use a parameter or an explicit process-scoped binding.
    pub const E_GLOBAL_UNSUPPORTED: Code = Code::new("E0204");
    /// `extract()`: introduces bindings whose names are not known statically.
    pub const E_EXTRACT_UNSUPPORTED: Code = Code::new("E0205");
    /// A PHP C extension that has no MWL equivalent.
    pub const E_UNSUPPORTED_EXTENSION: Code = Code::new("E0206");
    /// A `preg` pattern using a construct the pure-Rust engine cannot express.
    pub const E_UNSUPPORTED_REGEX: Code = Code::new("E0207");
    /// `static function`/`static fn`: closures already capture `$this` only
    /// if they use it, so `static` has nothing left to mean here.
    pub const E_STATIC_CLOSURE_UNSUPPORTED: Code = Code::new("E0210");
    /// A PHP superglobal (`$_GET`, `$_SERVER`, `$GLOBALS`, `$argv`, …): no
    /// variable is ever populated by the host — see ADR 0012.
    pub const E_SUPERGLOBAL_UNSUPPORTED: Code = Code::new("E0211");

    // --- E03xx name resolution ---------------------------------------------
    /// A variable read before anything was assigned to it.
    pub const E_UNDEFINED_VARIABLE: Code = Code::new("E0301");
    /// A call to a function that does not exist.
    pub const E_UNDEFINED_FUNCTION: Code = Code::new("E0302");
    /// A reference to a class, interface, trait or enum that does not exist.
    pub const E_UNDEFINED_CLASS: Code = Code::new("E0303");
    /// Two declarations with the same name in the same scope.
    pub const E_DUPLICATE_DECLARATION: Code = Code::new("E0304");
    /// A class hierarchy or trait use that forms a cycle.
    pub const E_CIRCULAR_INHERITANCE: Code = Code::new("E0305");
    /// A `use` statement that resolves to nothing.
    pub const E_UNRESOLVED_IMPORT: Code = Code::new("E0306");

    // --- E04xx types -------------------------------------------------------
    /// A value whose type cannot be what this position requires.
    pub const E_TYPE_MISMATCH: Code = Code::new("E0401");
    /// Wrong number of arguments.
    pub const E_ARITY_MISMATCH: Code = Code::new("E0402");
    /// A return type that no return statement can satisfy.
    pub const E_BAD_RETURN_TYPE: Code = Code::new("E0403");
    /// An override whose signature is not compatible with the parent's.
    pub const E_INCOMPATIBLE_OVERRIDE: Code = Code::new("E0404");
    /// A property or method access on a type that has no such member.
    pub const E_UNKNOWN_MEMBER: Code = Code::new("E0405");

    // --- E05xx IR and codegen ----------------------------------------------
    /// The IR verifier rejected a function. Always an MWL bug.
    pub const E_IR_INVALID: Code = Code::new("E0501");
    /// Cranelift could not compile a function.
    pub const E_CODEGEN_FAILED: Code = Code::new("E0502");

    // --- E06xx configuration and capabilities ------------------------------
    /// An `mwl.ini` directive that does not exist, or an invalid value.
    pub const E_BAD_DIRECTIVE: Code = Code::new("E0601");
    /// An `ini_set` the directive's changeability class refuses: a `System`
    /// directive, widening a `RuntimeTighten` one, or exceeding a hard ceiling.
    pub const E_CAPABILITY_DENIED: Code = Code::new("E0602");
    /// A per-request limit was exceeded.
    pub const E_LIMIT_EXCEEDED: Code = Code::new("E0603");

    // --- E09xx internal ----------------------------------------------------
    /// The compiler reached a state it believes impossible.
    pub const E_INTERNAL: Code = Code::new("E0901");

    // --- W1xxx warnings ----------------------------------------------------
    /// Code that can never execute.
    pub const W_UNREACHABLE: Code = Code::new("W1001");
    /// A variable assigned and never read.
    pub const W_UNUSED_VARIABLE: Code = Code::new("W1002");
    /// A deprecated construct that still works.
    pub const W_DEPRECATED: Code = Code::new("W1003");
    /// A construct whose behaviour differs from PHP's, where converted code may
    /// silently change meaning.
    pub const W_PHP_DIVERGENCE: Code = Code::new("W1004");
}
