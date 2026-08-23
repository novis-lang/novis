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
    /// `secret` applied to anything other than `string`/`bytes` — the
    /// qualifier's grammar restricts it to those two scalars, the same
    /// restriction `E_TAINTED_NON_SCALAR` enforces for `tainted`; see
    /// ADR 0033 § 1.
    pub const E_SECRET_NON_SCALAR: Code = Code::new("E0115");
    /// `tainted secret string`/`tainted secret bytes`: `secret` and `tainted`
    /// compose, but only in the order `secret` before `tainted` — see
    /// ADR 0033 § 1.
    pub const E_SECRET_TAINTED_ORDER: Code = Code::new("E0116");
    /// A `{name: value, ...}` object literal written where `{` already
    /// commits to a block — an expression-bodied `fn() => {...}`, or a bare
    /// statement-initial `{...}` — needs the same parenthesize-to-force-
    /// expression fix JavaScript uses for the identical ambiguity; see
    /// ADR 0036 § 2.
    pub const E_OBJECT_LITERAL_NEEDS_PARENS: Code = Code::new("E0117");
    /// `{x}` — an object literal has no shorthand; every field is written
    /// `name: value`. See ADR 0036 § 2.
    pub const E_OBJECT_LITERAL_SHORTHAND: Code = Code::new("E0118");
    /// `{[$expr]: value}` — an object literal has no computed/dynamic key;
    /// every field name is a static identifier. See ADR 0036 § 2.
    pub const E_OBJECT_LITERAL_COMPUTED_KEY: Code = Code::new("E0119");

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
    /// PHP's legacy `(T)expr` cast syntax — `as` is the only conversion
    /// spelling. See ADR 0034 § 1, which amends ADR 0007 § 2.
    pub const E_LEGACY_CAST_UNSUPPORTED: Code = Code::new("E0225");
    /// PHP's `and`/`or`/`xor` keyword operators — `&&`/`||` are the only
    /// logical connectives. See ADR 0045 §§ 1-2.
    pub const E_LOGICAL_KEYWORD_UNSUPPORTED: Code = Code::new("E0226");
    /// `trait Name { … }`, `use TraitName, ...;` inside a class body, or
    /// `insteadof` anywhere: traits do not exist — an interface
    /// default/private method replaces shared behavior, and
    /// `implements Interface by $field;` replaces shared state. See
    /// ADR 0043 §§ 1, 7. Replaces the narrower
    /// `E_TRAIT_METHOD_RENAME_UNSUPPORTED`/`E_TRAIT_METHOD_VISIBILITY_UNSUPPORTED`
    /// (both ADR 0015 § 3), now retired: there is no trait `use { ... }`
    /// adaptation grammar left to diagnose that finely, since traits do not
    /// exist at all. `E_TRAIT_METHOD_CONFLICT` (also ADR 0015 § 3) is
    /// retired for the same reason; the new default-method/delegation
    /// conflict diagnostic (`E_INTERFACE_MEMBER_CONFLICT`, ADR 0043 § 7)
    /// arrives with `mwl-hir`'s follow-up resolution work, not with this
    /// diagnostic.
    pub const E_TRAIT_NOT_SUPPORTED: Code = Code::new("E0227");
    /// `die`, in any position `exit` is also accepted: MWL keeps exactly one
    /// process-termination keyword. See ADR 0049 § 1.
    pub const E_DIE_UNSUPPORTED: Code = Code::new("E0228");
    /// The `<?php` open tag: MWL keeps exactly one code-mode open tag,
    /// `<?mwl` (plus the short-echo `<?=`). See ADR 0049 § 2.
    pub const E_PHP_OPEN_TAG_UNSUPPORTED: Code = Code::new("E0229");
    /// `list(...)` as a destructuring target: MWL keeps exactly one
    /// destructuring spelling, `[...]`. See ADR 0050.
    pub const E_LIST_DESTRUCTURING_UNSUPPORTED: Code = Code::new("E0230");
    /// A reserved lexical spelling written in anything but lower case —
    /// `<?MWL` rather than `<?mwl`. PHP matches its reserved spellings
    /// case-insensitively; MWL accepts exactly one spelling of each, so a
    /// program's meaning never depends on the case a reserved word was typed
    /// in. See [ADR 0062](../../../docs/adr/0062-case-sensitivity-is-a-compiler-property.md)
    /// § 2. A mis-cased *keyword* (`IF`, `TRUE`) gets no diagnostic of its
    /// own — it is simply an ordinary identifier, since ADR 0029 makes
    /// `IF` a legal class name the lexer cannot tell apart from a mis-typed
    /// `if`.
    pub const E_RESERVED_SPELLING_CASE: Code = Code::new("E0231");

    // --- E03xx name resolution ---------------------------------------------
    /// A variable read before anything was assigned to it.
    pub const E_UNDEFINED_VARIABLE: Code = Code::new("E0301");
    /// A call to a function that does not exist.
    pub const E_UNDEFINED_FUNCTION: Code = Code::new("E0302");
    /// A reference to a class, interface or enum that does not exist.
    pub const E_UNDEFINED_CLASS: Code = Code::new("E0303");
    /// Two declarations with the same name in the same scope.
    pub const E_DUPLICATE_DECLARATION: Code = Code::new("E0304");
    /// A class hierarchy (`extends`/`implements`) that forms a cycle.
    pub const E_CIRCULAR_INHERITANCE: Code = Code::new("E0305");
    /// A `use` statement that resolves to nothing.
    pub const E_UNRESOLVED_IMPORT: Code = Code::new("E0306");
    /// A `type` alias whose expression is nothing but one bare class,
    /// interface or enum atom — `use … as …` in disguise; see
    /// ADR 0015 § 6.
    pub const E_TYPE_ALIAS_ALIASES_CLASS: Code = Code::new("E0307");
    /// A `Class::member` reference (a static call, a class constant, an
    /// enum case, or a static property) names nothing declared on that class
    /// or any of its `extends`/`implements` ancestors — ADR 0011's "every
    /// callable and constant is a class member" has no bare-name fallback to
    /// fall into instead.
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
    /// A `require` whose literal path resolves on disk only because the
    /// filesystem is case-insensitive — `require 'mailer.mwl';` finding
    /// `Mailer.mwl`. Reported on Windows/macOS so the same source is not a
    /// `E_REQUIRE_TARGET_NOT_FOUND` on Linux; see
    /// [ADR 0062](../../../docs/adr/0062-case-sensitivity-is-a-compiler-property.md)
    /// § 3, which extends
    /// [ADR 0061](../../../docs/adr/0061-compile-time-autoload-and-program-discovery.md)
    /// § 1's exact-name rule from `autoload` to `require`.
    pub const E_REQUIRE_PATH_CASE_MISMATCH: Code = Code::new("E0314");

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
    /// `<`/`>`/`<=`/`>=`/`<=>` between two objects whose static types are not
    /// both provably the same class implementing the reserved global
    /// `Comparable` interface — either one side doesn't implement it, or the
    /// two sides are different classes even though both do; see ADR 0013
    /// §§ 3-4. PHP's implicit property-walk fallback has no MWL equivalent.
    pub const E_COMPARISON_REQUIRES_COMPARABLE: Code = Code::new("E0411");
    /// An object used at an implicit string-conversion site (interpolation,
    /// concatenation, `echo`/`print`, `as string`/`(string)`) whose static
    /// type does not provably implement the reserved global `Stringable`
    /// interface; see ADR 0028 § 1. PHP's own fallback here is already a
    /// fatal error, so nothing permissive is being removed.
    pub const E_STRINGABLE_REQUIRED: Code = Code::new("E0412");
    /// `unset()` on a declared object property, regardless of nullability —
    /// refused outright because ADR 0022 already guarantees no declared
    /// property is ever anything but definitely initialized; see ADR 0028
    /// § 3.
    pub const E_UNSET_ON_PROPERTY: Code = Code::new("E0413");
    /// `var $x = [...];` — a bare array literal has no target type to check
    /// against, the one initializer shape `var` cannot infer from; see
    /// ADR 0037 § 2.
    pub const E_VAR_ARRAY_LITERAL_NEEDS_TYPE: Code = Code::new("E0414");
    /// An arithmetic or bitwise operator applied directly to an enum-typed
    /// operand — neither is defined on an enum type; convert to its
    /// underlying `int`/`uint` with `as` first. See ADR 0010 § 5.
    pub const E_ENUM_ARITHMETIC_UNSUPPORTED: Code = Code::new("E0415");
    /// `as` from one enum type to a *different* enum type, even when both
    /// share the same underlying integer type — rejected outright; an
    /// explicit `match` naming every case is the replacement. See ADR 0010
    /// § 5.
    pub const E_ENUM_CONVERSION_UNSUPPORTED: Code = Code::new("E0416");
    /// `as Core\Html\Markup` on anything but a source-literal string — a
    /// runtime-computed or `tainted` value can never become trusted markup
    /// this way, closing "compute the escape-defeating payload at runtime,
    /// then cast it." See ADR 0024 § 5.
    pub const E_MARKUP_REQUIRES_LITERAL: Code = Code::new("E0417");
    /// A string passed (or convertible without laundering) where `callable`
    /// is the declared type — PHP's bare-name/`"Class::method"` callable
    /// spellings are both rejected in favor of first-class callable syntax.
    /// See ADR 0027 § 1.
    pub const E_CALLABLE_STRING_UNSUPPORTED: Code = Code::new("E0418");
    /// A `[$obj, 'method']`-shaped array passed where `callable` is the
    /// declared type. See ADR 0027 § 1.
    pub const E_CALLABLE_ARRAY_UNSUPPORTED: Code = Code::new("E0419");
    /// `$obj(...)` where `$obj`'s static type is not `callable` — MWL has no
    /// `__invoke`, so no class ever makes `()` mean anything else. See
    /// ADR 0027 § 1.
    pub const E_NOT_CALLABLE: Code = Code::new("E0420");
    /// A `secret`-qualified value reaching a `Core\Html\Markup`-building
    /// conversion — refused even though the equivalent `tainted`-only value
    /// would (once `Core\Html` exists) be auto-escaped instead: escaping
    /// neutralizes injection risk, not confidentiality. See ADR 0033 § 4.
    pub const E_SECRET_MARKUP_UNSUPPORTED: Code = Code::new("E0421");
    /// A `secret`-qualified value passed as a `Throwable`-shaped class's
    /// constructor message argument — closing the common leak of a
    /// credential ending up in a stack trace or an error page. See ADR 0033
    /// § 4.
    pub const E_SECRET_THROWABLE_MESSAGE: Code = Code::new("E0422");
    /// The `parent` type atom (`parent $x`, a parameter/property/return
    /// position — distinct from `new parent(...)`, which silently falls back
    /// to `mixed` for the same shape) used in a class with no `extends`.
    pub const E_NO_PARENT_CLASS: Code = Code::new("E0423");
    /// `lateinit` on a scalar-, enum-, or shape-typed property — only a
    /// class/interface (`object`-subtyped) property has no free real default
    /// for `lateinit` to defer past. See ADR 0038 § 1.
    pub const E_LATEINIT_NOT_OBJECT_TYPE: Code = Code::new("E0424");
    /// `lateinit` on a `?T` property — nullability already spells "may
    /// legitimately hold no value," so there is no second "not yet written"
    /// state left for `lateinit` to add. See ADR 0038 § 1.
    pub const E_LATEINIT_NULLABLE: Code = Code::new("E0425");
    /// `lateinit` on a promoted constructor parameter — binding the
    /// parameter is already the assignment ADR 0022 § 2 requires, so there is
    /// nothing left to defer. See ADR 0038 § 1.
    pub const E_LATEINIT_PROMOTED_PARAM: Code = Code::new("E0426");
    /// `lateinit` combined with `readonly` on the same property — opposite
    /// promises about when the one allowed assignment happens. See ADR 0038
    /// § 1.
    pub const E_LATEINIT_READONLY_CONFLICT: Code = Code::new("E0427");
    /// A `lateinit` property read inside a method body with no intervening
    /// write to it and no intervening call on that path since the method's
    /// entry — the one intraprocedural, false-positive-free case ADR 0038
    /// § 3 proves at compile time; every other case relies entirely on the
    /// § 2 runtime throw.
    pub const E_LATEINIT_READ_BEFORE_WRITE_LOCAL: Code = Code::new("E0428");
    /// An integer literal whose magnitude doesn't fit the width it's being
    /// checked against — too large for `int`/`uint` outright, or exactly the
    /// one magnitude `uint` can never represent regardless of width: a
    /// negative value, since an integer literal's digits are never signed and
    /// the sign comes from a wrapping unary `-`. See ADR 0007 § 4.
    pub const E_INT_LITERAL_OUT_OF_RANGE: Code = Code::new("E0429");
    /// A `\u{...}` escape inside a double-quoted string literal (or an
    /// interpolated-heredoc text run) names a value outside Unicode's valid
    /// scalar range (`> 0x10FFFF`) or inside the UTF-16 surrogate range
    /// (`0xD800..=0xDFFF`) — neither has a UTF-8 encoding, so it cannot be
    /// cooked into `string`'s guaranteed-valid-UTF-8 representation. Distinct
    /// from `E_INVALID_ESCAPE`, which the lexer already raises for a
    /// `\u{...}` that is syntactically malformed (no hex digits, or no
    /// closing `}`) — this fires only once the digits are syntactically fine
    /// but numerically out of range.
    pub const E_INVALID_UNICODE_ESCAPE: Code = Code::new("E0430");
    /// A double-quoted string literal's (or interpolated-heredoc text run's)
    /// `\xHH`/octal byte escapes assembled into a sequence that is not valid
    /// UTF-8 — `string` is guaranteed-valid UTF-8 (ADR 0009), so a byte
    /// escape's raw output has to actually decode, not just fit in a byte.
    pub const E_STRING_LITERAL_INVALID_UTF8: Code = Code::new("E0431");
    /// A heredoc/nowdoc's closing marker is indented with a mix of spaces and
    /// tabs — PHP 7.3's "flexible heredoc" rule (which this qualifier
    /// mirrors) requires the marker's own indentation to be one or the
    /// other, never both, since a body line's leading whitespace must match
    /// it byte-for-byte to be stripped. See
    /// `mwl_types::string_lit::heredoc_shape`.
    pub const E_HEREDOC_MIXED_INDENT: Code = Code::new("E0432");
    /// A non-blank heredoc/nowdoc body line has less leading whitespace than
    /// its own closing marker — PHP 7.3's "flexible heredoc" rule requires
    /// every body line to start with at least the marker's own indentation
    /// so it can be stripped uniformly. A line that is entirely empty is
    /// exempt from this check. See `mwl_types::string_lit::dedent_heredoc_run`.
    pub const E_HEREDOC_INSUFFICIENT_INDENT: Code = Code::new("E0433");
    /// A `float`, `bool`, or `null` array key — an explicit `key =>` in an
    /// array literal today, and eventually an `$a[...]` subscript once that
    /// site gains the identical check. PHP silently truncates a float,
    /// stringifies `true` to `"1"` and `null` to `""`; ADR 0007 § 5 rejects
    /// all three outright since each is a silent conversion at the exact
    /// place a mistake becomes a missing row. An `int`/`uint`/`string` key is
    /// fine — an `int`/`uint` key normalizes to its own decimal string, which
    /// needs no `as` and is not a value conversion.
    pub const E_ARRAY_KEY_INVALID_TYPE: Code = Code::new("E0434");
    /// A `private` interface method (ADR 0043 § 3) called from anywhere other
    /// than its own declaring interface's method bodies — it is an internal
    /// helper, never part of the interface's contract, so an implementing
    /// class (or any other interface) cannot see it at all, not even via
    /// `InterfaceName::method()`.
    pub const E_INTERFACE_PRIVATE_METHOD_NOT_VISIBLE: Code = Code::new("E0435");
    /// An `enum` case whose explicit `= expr` value is not an integer literal
    /// (or a negated one). ADR 0010 § 1 makes a case a compile-time integer
    /// constant, not a general constant-expression position.
    pub const E_ENUM_CASE_VALUE_NOT_LITERAL: Code = Code::new("E0436");
    /// An `enum` case whose value — written, or reached by ADR 0010 § 1's
    /// auto-increment — does not fit the enum's backing type.
    pub const E_ENUM_CASE_VALUE_OUT_OF_RANGE: Code = Code::new("E0437");
    /// `enum Name: T` where `T` is neither `int` nor `uint` — ADR 0010 § 2
    /// gives every enum exactly one underlying *integer* type. The `string`
    /// spelling has its own, earlier diagnostic
    /// ([`E_ENUM_STRING_BACKING_UNSUPPORTED`]); this covers the rest.
    pub const E_ENUM_BACKING_NOT_INTEGER: Code = Code::new("E0438");
    /// An argument passed to a `&$x` parameter that is not a *writable place*
    /// — a bare local or a compile-time-known property. A literal, an
    /// arithmetic result or a call's own result has no storage for the callee
    /// to write back into, so the reference would have nowhere to land.
    pub const E_BY_REF_ARG_NOT_A_PLACE: Code = Code::new("E0439");
    /// An argument passed to a `&$x` parameter whose type is not *exactly*
    /// the parameter's. ADR 0007 § 1 leaves no room for a conversion here:
    /// the callee writes back through the reference at the declared type, so
    /// anything the caller's storage would have to be converted from on the
    /// way in would have to be converted back on the way out — silently, and
    /// lossily.
    pub const E_BY_REF_ARG_TYPE_NOT_EXACT: Code = Code::new("E0440");
    /// A `<...>` type-argument list written after a name that takes no type
    /// parameters. ADR 0007 § 1 parks user-declared generics, and ADR 0053
    /// § 2 opens exactly one door in that wall — the compiler-owned generic
    /// interfaces `mwl_hir::interfaces::RESERVED` rosters. Everything else,
    /// including a `type` alias (ADR 0015 gives one no parameters of its
    /// own), lands here.
    pub const E_TYPE_ARGS_NOT_GENERIC: Code = Code::new("E0441");
    /// A compiler-owned generic interface written with the wrong number of
    /// type arguments, including none at all: `Iterator` on its own is as
    /// much a mistake as `Iterator<int, string>`, since the element type is
    /// the whole reason the parameter exists and ADR 0007 leaves no position
    /// untyped.
    pub const E_TYPE_ARG_COUNT: Code = Code::new("E0442");
    /// A `foreach` subject that is none of ADR 0053 § 3's three accepted
    /// shapes — an `array<T>`, an `Iterable<T>` or an `Iterator<T>`. A class
    /// reaching neither interface lands here, which is what keeps `foreach`
    /// from being a fourth implicit-dispatch site.
    pub const E_FOREACH_SUBJECT_NOT_ITERABLE: Code = Code::new("E0443");
    /// A `foreach ($x as $k => $v)` key binding over an `Iterable`/`Iterator`
    /// subject. ADR 0053 § 1 gives a cursor exactly `advance()` and
    /// `current()`; there is no key, and inventing a position counter would
    /// be a second thing `foreach` means.
    pub const E_FOREACH_KEY_ON_CURSOR: Code = Code::new("E0444");
    /// A `yield` in a body that is not a generator's own — at file scope, or
    /// inside an ADR 0031 closure. ADR 0053 § 4 confines `yield` lexically to
    /// the generator's own body, which is the stated price of lowering to a
    /// state machine rather than to a coroutine.
    pub const E_YIELD_OUTSIDE_GENERATOR: Code = Code::new("E0445");
    /// A generator — a function whose body contains `yield` — declaring a
    /// return type other than `Iterator<T>`. ADR 0053 § 4: calling one runs
    /// no user code and returns the state object, which implements exactly
    /// that interface.
    pub const E_GENERATOR_RETURN_TYPE: Code = Code::new("E0446");
    /// A `return expr;` inside a generator. ADR 0053 § 5 makes a generator a
    /// lazy sequence and nothing more — there is no generator return value to
    /// retrieve, so a bare `return;` (stop here) is the only form.
    pub const E_GENERATOR_RETURNS_A_VALUE: Code = Code::new("E0447");
    /// `yield from`, or a `yield` with a `key =>` half. ADR 0053 § 5 rejects
    /// the first as the second spelling of an explicit re-yield loop; § 1
    /// gives `Iterator<T>` no key for the second to produce.
    pub const E_YIELD_FORM_UNSUPPORTED: Code = Code::new("E0448");
    /// A concrete class that reaches an interface method nothing gives a
    /// body. Harmless while no syntax dispatched through an interface; ADR
    /// 0053 § 1's `Iterator<T>` made it a dispatch to nothing, since its
    /// members are bodiless by design.
    pub const E_INTERFACE_METHOD_MISSING: Code = Code::new("E0449");
    /// A block-bodied `fn` closure literal (ADR 0031 § 1) with no declared
    /// return type. An expression body *is* its own answer, so it needs no
    /// annotation; a block body would need whole-body return-type inference,
    /// which ADR 0007's "nothing is untyped, and no type ever changes by
    /// itself" does not ask the compiler to grow.
    pub const E_CLOSURE_RETURN_TYPE_REQUIRED: Code = Code::new("E0450");
    /// A parameter default (`function f(int $n = ...)`) that is not a literal
    /// of the parameter's own declared type, optionally negated. MWL evaluates
    /// a default once, at signature collection, and materializes it at the
    /// call site that omitted it — so it has to be a constant this compiler
    /// can emit, not PHP's general constant *expression*. See
    /// `mwl_types::defaults`, which owns the accepted set and the two shapes
    /// (`null`, an enum case) it is expected to grow next.
    pub const E_PARAM_DEFAULT_NOT_LITERAL: Code = Code::new("E0451");
    /// A parameter with no default declared *after* one that has a default.
    /// Every call supplies arguments positionally, so a required parameter
    /// behind an optional one could never be reached — PHP diagnoses the same
    /// shape.
    pub const E_PARAM_DEFAULT_ORDER: Code = Code::new("E0452");
    /// Something other than an ADR 0036 object literal written at a `Core`
    /// member's trailing options-bag parameter (ADR 0063 R2). The bag has no
    /// runtime representation — it flattens into one argument per declared
    /// option at the call site — so it must be written out there or omitted
    /// entirely; a variable holding one cannot be passed.
    pub const E_OPTIONS_NOT_A_LITERAL: Code = Code::new("E0453");
    /// A field name in an options bag that the member does not declare —
    /// usually a typo. Unlike ADR 0036 § 3's width subtyping, which accepts an
    /// extra field on purpose, an options bag refuses one: a misspelled option
    /// that is silently ignored is the failure ADR 0063 R2 exists to prevent.
    pub const E_UNKNOWN_OPTION: Code = Code::new("E0454");

    // --- E05xx IR and codegen ----------------------------------------------
    /// The IR verifier rejected a function. Always an MWL bug.
    pub const E_IR_INVALID: Code = Code::new("E0501");
    /// Cranelift could not compile a function.
    pub const E_CODEGEN_FAILED: Code = Code::new("E0502");

    // --- E06xx configuration and capabilities ------------------------------
    /// An `mwl.toml` directive that does not exist, or an invalid value.
    pub const E_BAD_DIRECTIVE: Code = Code::new("E0601");
    /// A `Core\Config::set` the directive's changeability class refuses: a `System`
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
