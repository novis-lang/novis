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
    /// Recursive-descent parsing nested past the recursion limit — malformed
    /// or adversarial input (e.g. thousands of nested `[`), never legitimate
    /// source. The parser bails out here rather than overflowing its stack.
    pub const E_TOO_DEEPLY_NESTED: Code = Code::new("E0108");
    /// `tainted` applied to anything other than `string`/`bytes` — the
    /// qualifier's grammar restricts it to those two scalars; see
    /// ADR 0024 § 1.
    pub const E_TAINTED_NON_SCALAR: Code = Code::new("E0109");
    /// A class/interface/trait/enum/enum-case/namespace-segment name is not
    /// `PascalCase` — ADR 0029's casing table.
    pub const E_BAD_TYPE_CASING: Code = Code::new("E0110");
    /// A method name is not `camelCase` — ADR 0029's casing table. Distinct
    /// from `E_LEGACY_CONSTRUCTOR_SPELLING`, which covers the one mis-cased
    /// spelling (`__construct`) that gets a targeted fix instead of this
    /// generic diagnostic.
    pub const E_BAD_METHOD_CASING: Code = Code::new("E0111");
    /// A property, parameter, local variable or closure self-name is not
    /// `camelCase` — ADR 0029's casing table, tightened by
    /// [ADR 0030](../../../docs/adr/0030-no-leading-underscores-constructor-spelling.md)
    /// § 1 to allow no leading underscore at all (ADR 0029's original
    /// one-underscore allowance for these three categories is revoked).
    pub const E_BAD_MEMBER_CASING: Code = Code::new("E0112");
    /// A class constant name is not `SCREAMING_SNAKE_CASE` — ADR 0029's
    /// casing table.
    pub const E_BAD_CONST_CASING: Code = Code::new("E0113");
    /// A method literally named `__construct` —
    /// [ADR 0030](../../../docs/adr/0030-no-leading-underscores-constructor-spelling.md)
    /// §§ 2-3: MWL's constructor is spelled `constructor`, an ordinary
    /// `camelCase` method name needing no exception of its own.
    pub const E_LEGACY_CONSTRUCTOR_SPELLING: Code = Code::new("E0114");

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
    /// `settype($x, ...)`: no assignment, operator or call can change what a
    /// binding's type is.
    pub const E_SETTYPE_UNSUPPORTED: Code = Code::new("E0208");
    /// `static $x = ...;` inside a function: there is no function-scope
    /// storage class — see ADR 0008.
    pub const E_STATIC_LOCAL_UNSUPPORTED: Code = Code::new("E0209");
    /// `static function`/`static fn`: closures already capture `$this` only
    /// if they use it, so `static` has nothing left to mean here.
    pub const E_STATIC_CLOSURE_UNSUPPORTED: Code = Code::new("E0210");
    /// A PHP superglobal (`$_GET`, `$_SERVER`, `$GLOBALS`, `$argv`, …): no
    /// variable is ever populated by the host — see ADR 0012.
    pub const E_SUPERGLOBAL_UNSUPPORTED: Code = Code::new("E0211");
    /// `use Path\To\Name as Other;`: an import cannot be renamed — see
    /// ADR 0015 § 2.
    pub const E_IMPORT_ALIAS_UNSUPPORTED: Code = Code::new("E0212");
    /// `Trait::method as newName;` inside a trait `use` block: a trait method
    /// cannot be renamed — see ADR 0015 § 3.
    pub const E_TRAIT_METHOD_RENAME_UNSUPPORTED: Code = Code::new("E0213");
    /// `Trait::method as public;` (or `protected`/`private`) inside a trait
    /// `use` block: a trait method's visibility cannot be changed by `as` —
    /// see ADR 0015 § 3.
    pub const E_TRAIT_METHOD_VISIBILITY_UNSUPPORTED: Code = Code::new("E0214");
    /// `function foo() { ... }` outside any class: a function must be a
    /// method — see ADR 0011 § 1.
    pub const E_TOPLEVEL_FUNCTION_UNSUPPORTED: Code = Code::new("E0215");
    /// `const FOO = 1;` outside any class: a constant must belong to a
    /// class — see ADR 0011 § 1.
    pub const E_TOPLEVEL_CONST_UNSUPPORTED: Code = Code::new("E0216");
    /// `namespace Core;` (or anything nested under it) in user source:
    /// `Core` is reserved for built-ins — see ADR 0011 § 2.
    pub const E_RESERVED_CORE_NAMESPACE: Code = Code::new("E0217");
    /// `enum Name implements Iface { ... }`: an enum declares only cases and
    /// an optional backing type — see ADR 0010 § 3.
    pub const E_ENUM_IMPLEMENTS_UNSUPPORTED: Code = Code::new("E0218");
    /// `enum Name: string { ... }`: no `string` backing, only `int`/`uint` —
    /// see ADR 0010 § 3.
    pub const E_ENUM_STRING_BACKING_UNSUPPORTED: Code = Code::new("E0219");
    /// A method, property, class constant or trait use inside an `enum`
    /// body: an enum declares only cases and an optional backing type — see
    /// ADR 0010 § 3.
    pub const E_ENUM_MEMBER_UNSUPPORTED: Code = Code::new("E0220");
    /// `include`, `include_once`, or `require_once`: MWL keeps exactly one
    /// same-frame inclusion construct, `require` — see ADR 0021.
    pub const E_INCLUDE_FAMILY_UNSUPPORTED: Code = Code::new("E0221");
    /// An anonymous `function (...) { ... }` literal, with or without a
    /// `use` clause: `fn` is the only closure literal — see ADR 0031 § 1.
    pub const E_FUNCTION_CLOSURE_UNSUPPORTED: Code = Code::new("E0222");
    /// `use ($y)` on a closure literal: capture is always implicit and by
    /// value, so there is no clause to write — see ADR 0031 § 2.
    pub const E_CLOSURE_USE_UNSUPPORTED: Code = Code::new("E0223");
    /// `use (&$y)` on a closure literal specifically: by-reference capture
    /// has no replacement syntax — see ADR 0031 § 2.
    pub const E_CLOSURE_USE_BY_REF_UNSUPPORTED: Code = Code::new("E0224");

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
    /// A `type` alias whose expression is nothing but one bare class,
    /// interface or enum atom — `use … as …` in disguise; see
    /// ADR 0015 § 6.
    pub const E_TYPE_ALIAS_ALIASES_CLASS: Code = Code::new("E0307");
    /// Two traits used by the same class/trait declare the same method name,
    /// and no `insteadof` names a winner; see ADR 0015 § 3.
    pub const E_TRAIT_METHOD_CONFLICT: Code = Code::new("E0308");
    /// A `Class::member` reference (a static call, a class constant, an
    /// enum case, or a static property) names nothing declared on that class
    /// or any of its `extends`/`implements`/trait-use ancestors — ADR 0011's
    /// "every callable and constant is a class member" has no bare-name
    /// fallback to fall into instead.
    pub const E_UNDEFINED_MEMBER: Code = Code::new("E0309");
    /// A `type` alias whose expansion, followed far enough, refers back to
    /// itself — `type A = B; type B = A;` or any longer cycle; see
    /// ADR 0015 § 5.
    pub const E_TYPE_ALIAS_CYCLE: Code = Code::new("E0310");
    /// A `require` whose path is a literal, resolved statically per
    /// ADR 0021, but does not name a file that can be loaded as source (it
    /// does not exist, or is not valid UTF-8).
    pub const E_REQUIRE_TARGET_NOT_FOUND: Code = Code::new("E0311");
    /// A `require` chain whose statically-resolved literal paths lead back
    /// to a file already being resolved — `require`'s own semantics (same
    /// frame, runs every time reached) give this no other resolution than a
    /// diagnostic, the same way `crate::hierarchy`'s `extends`/trait-use
    /// cycle and `crate::aliases`'s `type` alias cycle are both handled.
    pub const E_CIRCULAR_REQUIRE: Code = Code::new("E0312");
    /// `$this->name` where `name` is not declared on the enclosing class or
    /// any `extends`/`implements`/trait-use ancestor — ADR 0014 § 5's "no
    /// `__get`/`__set` fallback" for the one receiver shape resolvable
    /// without a type checker.
    pub const E_UNDEFINED_PROPERTY: Code = Code::new("E0313");

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
    /// A local variable declared a second time while its first declaration
    /// is still live — ADR 0007 § 1: "there is no shadowing."
    pub const E_REDECLARED_LOCAL: Code = Code::new("E0406");
    /// `int ⊕ uint` arithmetic — ADR 0007 § 4: there is no representable
    /// common type, so one side must be converted explicitly.
    pub const E_INT_UINT_ARITHMETIC: Code = Code::new("E0407");
    /// An `array<...>` type nests past ADR 0007 § 5's depth-32 bound.
    pub const E_ARRAY_TYPE_TOO_DEEP: Code = Code::new("E0408");
    /// A non-nullable, no-default property a class declares (itself, or
    /// through a used trait) is not definitely assigned on some path out of
    /// its constructor — or the class has no constructor at all to assign
    /// it; see ADR 0022 § 2.
    pub const E_UNINITIALIZED_PROPERTY: Code = Code::new("E0409");
    /// A subclass constructor has a path that never calls
    /// `parent::constructor(...)`, so the properties it inherits are never
    /// discharged on that path; see ADR 0022 § 2.
    pub const E_MISSING_PARENT_CONSTRUCTOR_CALL: Code = Code::new("E0410");

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
