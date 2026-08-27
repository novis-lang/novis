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
    /// A duration literal that does not follow
    /// [ADR 0070](../../../docs/adr/0070-duration-literals.md) § 1's grammar —
    /// out of order, a repeated unit, a fractional count, a mis-cased unit, or
    /// longer than `Core\Time\Duration` can hold.
    pub const E_BAD_DURATION_LITERAL: Code = Code::new("E0007");
    /// A bidirectional control that opens a directional scope and never closes
    /// it inside the source span that opened it, per
    /// [ADR 0087](../../../docs/adr/0087-unbalanced-bidi-is-rejected-at-every-boundary.md)
    /// § 2 — a comment, a string literal or an inline-HTML run, and each line
    /// of a multi-line one. There is no suppression.
    pub const E_UNBALANCED_BIDI: Code = Code::new("E0008");
    /// An `<?mwl` open tag in a file that opens with `#!` and is therefore
    /// already in code mode, before any `?>` has left it, per
    /// [ADR 0100](../../../docs/adr/0100-against-python-mwl-claims-the-tool-that-gets-handed-over.md)
    /// § 3. Reserved by that ADR and reported once its lexer slice lands.
    pub const E_TAG_IN_SHEBANG_FILE: Code = Code::new("E0009");

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
    /// A `float` literal in type position — ADR 0047 § 7 defers float literal
    /// types until floating-point equality has a real answer, so `0.1` names
    /// no type the way `1` and `"a"` now do.
    pub const E_FLOAT_LITERAL_TYPE: Code = Code::new("E0120");
    /// An interpolated string in type position — `"a"` is ADR 0047 § 1's
    /// singleton type, and a type has no scope to interpolate a variable
    /// from.
    pub const E_INTERPOLATION_IN_TYPE: Code = Code::new("E0121");
    /// A member declaration — property, class constant or method, in a
    /// `class`, `interface` or anonymous-class body — carrying no
    /// `public`/`protected`/`private`, or PHP 8.4's `(set)` form written
    /// without its read visibility. There is no implicit `public`; see
    /// [ADR 0094](../../../docs/adr/0094-visibility-is-written-at-every-member-declaration.md).
    /// A class body's PHP `var $x;` reports this too, rather than a message
    /// about the statement grammar it would otherwise fall into (§ 4).
    pub const E_MISSING_VISIBILITY: Code = Code::new("E0122");
    /// An `autoload` prefix, root or glob written as anything but a plain
    /// string literal — an interpolated `"$dir"`, a concatenation, a
    /// variable. Every path resolves at compile time, relative to the file
    /// the declaration appears in, so there is nothing to interpolate from;
    /// see [ADR 0061](../../../docs/adr/0061-compile-time-autoload-and-program-discovery.md)
    /// § 1, which carries `require`'s literal-only restriction for the same
    /// reason.
    pub const E_AUTOLOAD_PATH_NOT_LITERAL: Code = Code::new("E0123");

    // --- E02xx rejected PHP constructs -------------------------------------
    // MWL accepts PHP 8.5 syntax as a *pragmatic* superset. These constructs
    // are recognised — so the diagnostic can be precise and suggest a
    // replacement — and are then rejected. Most parse first; the two that a
    // *lexical* rule refuses (`E_RESERVED_SPELLING_CASE`,
    // `E_IDENTITY_OPERATOR_UNSUPPORTED`) are named where they are recognised,
    // which is the lexer. See docs/spec.
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
    /// PHP's `===`/`!==` — MWL keeps exactly one equality operator, `==`, and
    /// its negation `!=`. See
    /// [ADR 0090](../../../docs/adr/0090-one-equality-operator-and-disjoint-types-do-not-compile.md)
    /// § 1. This is the one construct in this band the *lexer* reports rather
    /// than the parser: there is nothing to parse precisely here, so the
    /// three characters are consumed, named, and lexed as the two-character
    /// operator so the rest of the file still reports its own problems.
    pub const E_IDENTITY_OPERATOR_UNSUPPORTED: Code = Code::new("E0232");

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
    /// Two `autoload` declarations in one program claim the same namespace
    /// prefix — [ADR 0061](../../../docs/adr/0061-compile-time-autoload-and-program-discovery.md)
    /// § 1's "one prefix has one home". An explicit prefix deliberately does
    /// *not* collide with a `discover` glob that would produce the same one:
    /// there the glob skips the name, which is what makes a vendor override
    /// work.
    pub const E_DUPLICATE_AUTOLOAD_PREFIX: Code = Code::new("E0315");
    /// An `autoload` declaration in a file that was itself reached through
    /// the autoload map — ADR 0061 § 1, which honors a declaration only in a
    /// file reachable by `require` from the entry point, since otherwise the
    /// map would depend on itself.
    pub const E_AUTOLOAD_IN_AUTOLOADED_FILE: Code = Code::new("E0316");
    /// A file reached through an autoload root that does not hold exactly one
    /// top-level declaration named after it — ADR 0061 § 2. Without the rule,
    /// whether a name exists in the program depends on what was resolved
    /// first, which makes the build non-reproducible and the cache unkeyable.
    pub const E_AUTOLOAD_FILE_SHAPE: Code = Code::new("E0317");
    /// An `autoload discover` glob that is not one `*` occupying a whole path
    /// segment, or whose base directory does not exist — ADR 0061 § 1. A
    /// *matched* directory with an unusable name is skipped in silence (a
    /// glob over a filesystem always sweeps `.git` and `vendor`); the glob
    /// itself is diagnosed, since one that silently discovers nothing is the
    /// worst outcome on offer.
    pub const E_AUTOLOAD_GLOB_SHAPE: Code = Code::new("E0318");
    /// A bare name used where a value is expected — `PHP_EOL`, `MY_LIMIT` —
    /// which in PHP would be a global constant fetch.
    /// [ADR 0011](../../../docs/adr/0011-functions-and-constants-are-class-members.md)
    /// § 3 removed that storage row outright: a constant is always a class
    /// constant, so there is no name for this to resolve against and nothing
    /// below the resolver to lower it to.
    pub const E_NO_GLOBAL_CONSTANT: Code = Code::new("E0319");
    /// A bare name called as a function — `strlen($s)` — which in PHP would
    /// be a global function call. ADR 0011 § 1: every callable is a method,
    /// with no exception for built-ins, which live under the reserved `Core`
    /// namespace. Split from [`E_NO_GLOBAL_CONSTANT`] because the two carry
    /// different replacements even though the callee is the same node.
    pub const E_NO_FREE_FUNCTION: Code = Code::new("E0320");
    /// `self`, `static` or `parent` written where a value is expected, rather
    /// than on the left of a `::`. Each of the three names a *class*, and a
    /// class is not a value in MWL — there is no class-object reflection
    /// handle ([ADR 0011](../../../docs/adr/0011-functions-and-constants-are-class-members.md)
    /// puts every reflective question on `Core\Reflect` instead).
    pub const E_CLASS_NAME_NOT_A_VALUE: Code = Code::new("E0321");

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
    /// parameters. ADR 0007 § 1 parks user-declared generics, and two doors
    /// open in that wall — ADR 0053 § 2's compiler-owned generic interfaces,
    /// which `mwl_hir::interfaces::RESERVED` rosters, and a `Core` member
    /// whose spec signature writes one (`Core\Json::decodeAs<T>`), which
    /// `mwl_stdlib::registry::CoreTy::Written` marks. Everything else lands
    /// here: a `type` alias (ADR 0015 gives one no parameters of its own), a
    /// user-declared method, and a `Core` member that infers its variables
    /// from its arguments instead.
    pub const E_TYPE_ARGS_NOT_GENERIC: Code = Code::new("E0441");
    /// A name or member written with the wrong number of type arguments,
    /// including none at all: `Iterator` on its own is as much a mistake as
    /// `Iterator<int, string>`, since the element type is the whole reason the
    /// parameter exists and ADR 0007 leaves no position untyped. A call site
    /// that omits a member's required list reaches the same rule.
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
    /// `decimal ⊕ float` arithmetic, or `**` with a `decimal` base — ADR 0054
    /// § 3. The same rule and the same reason as [`E_INT_UINT_ARITHMETIC`]:
    /// there is no type that represents both operands' values, so one side
    /// must be converted explicitly.
    pub const E_DECIMAL_FLOAT_ARITHMETIC: Code = Code::new("E0455");
    /// A numeric literal placed at `decimal` whose mantissa exceeds 96 bits or
    /// whose scale exceeds 28 — ADR 0054 § 1's layout. `Core\BigDecimal` (§ 6)
    /// is the type for a value beyond it.
    pub const E_DECIMAL_LITERAL_OUT_OF_RANGE: Code = Code::new("E0456");
    // `E0457` (`E_LITERAL_TYPE_UNCHECKED`) is **retired**, not reused.
    // [ADR 0047](../../../docs/adr/0047-literal-and-enum-case-types.md)'s three
    // atoms intern as real types now (`mwl_types::lower::lower_atom`), so there
    // is nothing left for it to refuse.
    /// A `Core` **instance** member written as a static call —
    /// `Core\Regex\Match::text($m)` rather than `$m->text()`. ADR 0063 R20
    /// gives every `Core` operation exactly one spelling, and this is the one
    /// place two could otherwise reach the same helper: an instance member's
    /// receiver is argument slot 0 at the ABI, so the static spelling would
    /// pass the arity check with the receiver written as an ordinary argument.
    pub const E_CORE_INSTANCE_MEMBER_CALLED_STATICALLY: Code = Code::new("E0458");
    /// A plain `->` on a receiver whose type includes `null` — `?->`, or a
    /// `!= null` test around it, is how a member of one is reached. PHP
    /// throws for this at run time; MWL refuses it while compiling, because
    /// `?T` is one union with no class to resolve a member against. Inside a
    /// block a `!= null`/`== null` test proved the receiver non-`null`
    /// (`mwl_types::locals`' narrowing) this does not fire at all — until
    /// something in that block assigns the local again, which takes the
    /// narrowing back off.
    pub const E_NULLABLE_RECEIVER: Code = Code::new("E0459");
    /// A `#[Json\Derive]` field that is not a same-named constructor parameter.
    /// ADR 0071 § 2 makes a decode an ordinary `new`, so every field the codec
    /// reads has to have a parameter to arrive through; `#[Json\Field(skip:
    /// true)]` is the stated way out.
    pub const E_DERIVE_FIELD_NOT_A_PARAMETER: Code = Code::new("E0460");
    /// A `#[Json\Derive]` field whose constructor parameter is declared with a
    /// different type than the property. ADR 0071 § 2: the two lists are one
    /// declaration for a promoted parameter, so a divergence is always written
    /// by hand and always a mistake.
    pub const E_DERIVE_FIELD_TYPE_MISMATCH: Code = Code::new("E0461");
    /// A `secret` property on a class carrying `#[Json\Derive]`. ADR 0071 § 6
    /// moves ADR 0033's refusal from wherever the value reached the encoder to
    /// the declaration that put it on the wire contract.
    pub const E_DERIVE_SECRET_FIELD: Code = Code::new("E0462");
    /// A `lateinit` property on a class carrying `#[Json\Derive]`. ADR 0071
    /// § 2: `lateinit` (ADR 0038) is by definition not constructor-assigned,
    /// so it can never be a field.
    pub const E_DERIVE_LATEINIT_FIELD: Code = Code::new("E0463");
    /// A `#[Json\Field(...)]` argument that is not one of ADR 0071 § 3's two
    /// options, or whose value is not a literal of that option's type.
    pub const E_DERIVE_FIELD_ATTRIBUTE: Code = Code::new("E0464");
    /// A type argument written where the member needs a *class* rather than
    /// any type — `Core\Json::decodeAs<int>`. The members that do are
    /// `mwl_stdlib::registry::WRITTEN_CLASS_MEMBERS`, and each of them reaches
    /// the written class's runtime descriptor from native code, which only a
    /// class has.
    pub const E_TYPE_ARG_NOT_A_CLASS: Code = Code::new("E0465");
    /// An `==`/`!=` — or a `switch` label, or a `match` arm — whose two static
    /// types are **disjoint**: no single value inhabits both, so the compiler
    /// already knows the answer. ADR 0090 § 2's table, and § 6 for the two
    /// comparison forms that are not written with the operator.
    pub const E_DISJOINT_EQUALITY: Code = Code::new("E0466");
    /// `+` or `+=` with an array operand. ADR 0069 § 2 removes PHP's array
    /// union operator rather than migrating it — the diagnostic names
    /// `Core\Arr::underlay`, which is what it always meant.
    pub const E_ARRAY_PLUS_UNSUPPORTED: Code = Code::new("E0467");
    /// `Foo::BAR` in *type* position where `Foo::BAR` is declared but is not a
    /// `string`/`int` compile-time constant — ADR 0047 § 2. A class constant is
    /// sugar that folds to its own literal type, so it folds only when the
    /// value has a literal type to fold to: a `float` (§ 7 defers those), an
    /// `array`, an object, or an expression that is not a literal at all has
    /// none. A name nothing declares is [`E_UNKNOWN_MEMBER`] instead — that is
    /// a different mistake with a different fix.
    pub const E_LITERAL_TYPE_NOT_CONST: Code = Code::new("E0468");
    /// `"z" as "a"|"b"` — ADR 0047 § 6: a checked conversion into a closed set
    /// of literals whose operand already names a value the set does not
    /// contain, so it would compile and then throw on every execution. The
    /// accepted set in the message is generated from the target type, never
    /// written per site. A conversion the operand does not settle — `$s as
    /// "a"|"b"` over a plain `string` — is § 4's ordinary checked row and
    /// compiles.
    pub const E_LITERAL_TYPE_MISMATCH: Code = Code::new("E0469");
    /// `Mode::Admin as Mode::Read|Mode::Write` — § 3's case-subset half of
    /// [`E_LITERAL_TYPE_MISMATCH`], on the same terms. Its own code because
    /// the set it names is a set of *cases* rather than of literal values,
    /// which is the distinction § 3 exists to keep.
    pub const E_ENUM_CASE_SUBSET_MISMATCH: Code = Code::new("E0470");
    /// `$obj->secret` where `secret` is declared `private` outside the class
    /// the access is written in, or `protected` outside that class and its
    /// subclasses — ADR 0094's levels, now meaning something. The test is
    /// keyed on the **accessing** class and never on the receiver's static
    /// type: `$other->secret` is legal inside `Secret`'s own body and the
    /// identical line is not at file scope. A name nothing declares anywhere
    /// in the chain is [`E_UNKNOWN_MEMBER`] instead.
    pub const E_MEMBER_NOT_VISIBLE: Code = Code::new("E0471");
    /// `public int $n = "no";` — a property's inline default is evaluated once,
    /// at signature collection, into the constant every fresh instance's slot
    /// is written with (`mwl_types::defaults`), so it has to be a literal of
    /// the property's own declared type. Its own code rather than
    /// [`E_PARAM_DEFAULT_NOT_LITERAL`] because the two accept different sets:
    /// a property may be defaulted to `[]` and a parameter may not.
    pub const E_PROPERTY_DEFAULT_NOT_LITERAL: Code = Code::new("E0472");
    /// `$obj as ?SomeClass` — ADR 0066 § 3's class row: `instanceof` plus
    /// ADR 0007 § 6's narrowing already answers class membership, so the
    /// conversion would be R17's second spelling of a question the language
    /// already has one for. Reported for the written `?T` sugar only, since
    /// § 1 deliberately leaves the `SomeClass|null` union spelling out of the
    /// form.
    ///
    /// **There are no exceptions**, including `Core\Uri` and `Core\Uuid`.
    /// An earlier revision of that ADR admitted those two as a *parse roster*
    /// where `$s as ?Core\Uri` compiled; § 3 withdrew it, and § 3a's
    /// `Core\Uri::tryParse` is the member that answers a parse instead — which
    /// the help names for a class in `mwl_stdlib::registry::TRY_PARSE_CLASSES`.
    pub const E_CLASS_CONVERSION_TARGET: Code = Code::new("E0473");
    /// `++`/`--` on a binding that is not one of ADR 0007 § 4's numeric
    /// types.
    ///
    /// An increment is `± 1` and nothing else. PHP's `$s++` walking a string
    /// through `"a"`→`"b"`→`"aa"` is a divergence this takes deliberately:
    /// § 2 says a declared type never changes and § 4's table has no row that
    /// produces `"b"` from a `string` and a `1`, so there is no arithmetic
    /// here to lower. `mwl_types::expr::operators`' module doc is that
    /// decision's home.
    pub const E_INCREMENT_NOT_NUMERIC: Code = Code::new("E0474");
    /// A `break`/`continue` whose level names no target it can jump to: a
    /// level computed at run time, a `0`, or more enclosing loops than there
    /// are — including the bare `break;` written outside every loop.
    ///
    /// PHP refuses all four at compile time too, and for the same reason:
    /// `break N` resolves to a *statically known* enclosing statement, so a
    /// level that names none has nothing to lower to.
    /// `mwl_types::locals` is where the enclosing depth is counted.
    pub const E_BREAK_LEVEL: Code = Code::new("E0475");
    /// A `match` written with no arms at all.
    ///
    /// PHP parses one and throws `UnhandledMatchError` on every evaluation,
    /// so the construct has no reachable value there either. MWL refuses it
    /// where it is written instead: a `match` is an *expression*, and one
    /// whose every path throws has nothing for the position it sits in to
    /// bind, pass or return. Nothing that worked is lost — a written arm, or
    /// a `throw` expression, says the same thing and says it on purpose.
    pub const E_MATCH_NO_ARMS: Code = Code::new("E0476");
    /// A method called on a receiver whose type names no class: a plain
    /// `object` (ADR 0007 § 3's opaque top of every class type) or an
    /// ADR 0036 shape, neither of which lists a single method.
    ///
    /// ADR 0036 § 4 gave the *property* half of an erased receiver a
    /// name-keyed runtime fetch, and deliberately stopped there. A call needs
    /// an argument list checked against a signature and a return type to bind
    /// the position it sits in, and an erased receiver supplies neither — so
    /// there is nothing here to resolve, and no `__call` to fall back on
    /// (ADR 0014). Narrow first: `instanceof` proves the class, and
    /// `as ClassName` converts to it.
    pub const E_METHOD_ON_ERASED_RECEIVER: Code = Code::new("E0477");
    /// An array element written through an ADR 0014 § 1 hooked property:
    /// `$obj->hooked[0] = v`, or any deeper subscript over the same base.
    ///
    /// A hooked property is a pair of accessors, not a slot, so the element
    /// write has nothing to write *into*: ADR 0007 § 5 separates the array
    /// the `get` hook returned, and the separated copy would then have to be
    /// pushed back through `set`, which PHP does not do either — it raises
    /// "indirect modification of overloaded property" and discards the write.
    /// Refusing where it is written is therefore the PHP-compatible answer as
    /// well as the honest one. Read the array into a local, write the
    /// element, and assign the local back through the property.
    pub const E_ELEMENT_WRITE_THROUGH_HOOK: Code = Code::new("E0478");
    /// A nullsafe access used as an assignment target: `$a?->b = v`.
    ///
    /// `?->` means "or `null`", and `null` is not a place: PHP refuses the
    /// same spelling with "can't use nullsafe operator in write context",
    /// because the alternative is an assignment that silently does nothing
    /// on the null path. Test the receiver instead — `if ($a !== null) {
    /// $a->b = v; }` says which of the two outcomes was meant.
    pub const E_NULLSAFE_WRITE_TARGET: Code = Code::new("E0479");
    /// An array element written through a property whose receiver erased:
    /// an ADR 0036 shape, or ADR 0007 § 3's plain `object`.
    ///
    /// ADR 0036 § 4 gave such a property a name-keyed runtime *fetch* and
    /// deliberately stopped there, which is enough for a read and not enough
    /// for a write: ADR 0007 § 5 separates the array on the way in, and the
    /// separated copy needs a slot to be written back into that a by-name
    /// resolution does not supply. A plain `object` receiver has no element
    /// type either — the property reads as `mixed`, and `mixed` is not
    /// indexable anywhere else in the language. Narrow the receiver first
    /// (`instanceof`, or `as ClassName`), or read the property into a typed
    /// local, write the element there, and assign it back. This is the
    /// element-write twin of [`E_METHOD_ON_ERASED_RECEIVER`].
    pub const E_ELEMENT_WRITE_THROUGH_ERASED_PROPERTY: Code = Code::new("E0480");
    /// `$a[]` where a value is read rather than assigned to.
    ///
    /// `[]` names "the key one past the highest integer key" and only has
    /// an answer as the *destination* of a write: read it and there is no
    /// element there to read. PHP refuses the identical spelling at compile
    /// time with "Cannot use [] for reading", so this is not a divergence.
    /// Every position but the target of a plain `=` is a read, which is why
    /// `unset($a[])` takes this code too — PHP refuses that one as well,
    /// with its own wording. An append at an intermediate level of a write
    /// target (`$a[][0] = 1`) is a target and stays legal.
    ///
    /// `$a[] .= "x"` is the one spelling PHP accepts and this refuses, and
    /// it is ADR 0007 § 7 row 10: PHP appends because the element that is
    /// not there yet reads as `""`, and no rule in MWL makes an absent
    /// element read as a zero value — which is § 7 row 8 one storage kind
    /// along, not a new judgement.
    pub const E_APPEND_IN_READ_POSITION: Code = Code::new("E0481");
    /// `$x[…]` where `$x` is not an `array<T>`.
    ///
    /// ADR 0007 § 5 checks an element read and write against the array's
    /// *declared* element type, so a base that declares none — a `mixed`, a
    /// scalar, an object, or a `?array<T>` a `!== null` test has not
    /// narrowed — has no element to name and nothing to check against.
    /// PHP answers `null` with a warning for most of these, which is § 7
    /// row 8's family; MWL refuses at check time instead. A `string` is not
    /// an exception: ADR 0009 § 2 indexes one by grapheme cluster through
    /// `Core\Str`, not through a subscript.
    pub const E_SUBSCRIPT_ON_NON_ARRAY: Code = Code::new("E0482");
    /// `&value` as an element of an array literal.
    ///
    /// PHP's `[&$x]` stores a reference, so writing the element writes
    /// `$x` too. MWL has nowhere to put one: ADR 0031 § 2 removed
    /// by-reference capture, so no binding aliases another, and ADR 0023
    /// fixes what a copy means, so an element is a copy at the point the
    /// literal is evaluated. An aliasing element would therefore have no
    /// owner in either rule — it is not a lowering that is missing, it is
    /// a thing the language does not have. Write the value; to share one
    /// mutable cell, put it in an object, exactly as ADR 0031 § 2's own
    /// worked example does.
    pub const E_ARRAY_ELEMENT_BY_REFERENCE: Code = Code::new("E0483");
    /// `[...$x]` where `$x` is not an `array<T>`.
    ///
    /// A spread element contributes the subject's *entries* to the literal
    /// being built, so a subject with no entries has nothing to contribute.
    /// PHP's `[...$s]` over a string is a `TypeError` at run time; MWL's
    /// element types are declared, so it is a diagnostic instead. Where the
    /// literal does have an expected element type the mismatch is reported
    /// as an ordinary [`E_TYPE_MISMATCH`] against `array<T>` instead, which
    /// names both array types and is the better message — so this code is
    /// only what a position naming no `array<T>` at all is left with, a
    /// `mixed` binding or parameter being the reachable one.
    pub const E_SPREAD_SUBJECT_NOT_AN_ARRAY: Code = Code::new("E0484");
    /// `name: value` at a call whose target names none of its parameters.
    ///
    /// A `Core` member's parameters are types in `mwl_stdlib::registry` and
    /// nothing else — the rows carry no names, deliberately, because ADR 0063
    /// R2 already gives every `Core` member its by-name surface as a trailing
    /// options bag (`{limit: 4}`). So there is no name to call one by, and
    /// inventing one at the registry would be a second spelling of the same
    /// optional argument. The synthesized `Throwable` constructor is the other
    /// signature with no names.
    pub const E_NAMED_ARG_NO_PARAM_NAMES: Code = Code::new("E0485");
    /// `name: value` naming no parameter a call can fill by name — either the
    /// callee declares no parameter of that name at all, or the name reaches
    /// its `...$rest` tail.
    ///
    /// PHP collects an unmatched named argument into a variadic parameter as a
    /// string-keyed entry. MWL's variadic tail is an ordinary `array<T>` built
    /// at the call site from the arguments written into it, so a name has
    /// nowhere to be recorded — and a caller that wants a keyed entry writes
    /// the array itself.
    pub const E_UNKNOWN_ARG_NAME: Code = Code::new("E0486");
    /// One parameter given an argument twice — positionally and then by name,
    /// or by the same name twice.
    pub const E_DUPLICATE_ARG: Code = Code::new("E0487");
    /// A positional argument after a `name:` or a `...` one.
    ///
    /// PHP refuses both orderings for the same reason: which parameter a
    /// positional argument fills is its own position in the list, and neither
    /// a named argument nor an unpacked array leaves that position defined.
    pub const E_POSITIONAL_AFTER_NAMED: Code = Code::new("E0488");
    /// `...$rest` at a call with no variadic parameter left for it to land in
    /// — the callee declares no `...$x` at all, or one or more of its fixed
    /// parameters is still unfilled where the spread is written.
    ///
    /// How many entries an array holds is a run-time fact, so a spread that
    /// had to fill fixed parameters would leave a call's arity uncheckable.
    /// Write the fixed arguments out and let the spread supply the tail.
    pub const E_SPREAD_ARG_NOT_VARIADIC: Code = Code::new("E0489");
    /// `foreach (… as &$v)` over a subject that is not a plain variable
    /// holding an `array<T>` — a call's result, a literal, a property, or an
    /// `Iterable`/`Iterator`.
    ///
    /// `&$v` writes each element back where it came from, so there has to be
    /// a slot to write to. PHP refuses the same shapes, and a cursor is the
    /// one it names outright ("an iterator cannot be used with foreach by
    /// reference").
    pub const E_FOREACH_BY_REF_SUBJECT: Code = Code::new("E0490");
    /// A `foreach (… as T &$v)` whose `T` is not the subject's element type
    /// exactly.
    ///
    /// A by-value binding may widen — reading an `array<Dog>` as an `Animal`
    /// is sound — but a by-reference one also *writes*, and writing an
    /// `Animal` into an `array<Dog>` is not. The two directions meet only at
    /// the element type itself.
    pub const E_FOREACH_BY_REF_ELEMENT_TY: Code = Code::new("E0491");
    /// A method whose body contains `yield` declaring a `&$x` parameter.
    ///
    /// A by-reference parameter addresses a cell the *call site* stages for
    /// the duration of the call. Calling a generator runs none of its body —
    /// it allocates the state object and returns (ADR 0053 § 4) — so that cell
    /// is gone before the first `advance()`, and there is nothing sound for
    /// the suspended frame to keep addressing.
    pub const E_GENERATOR_BY_REF_PARAM: Code = Code::new("E0492");
    /// A `fn` closure literal declaring a `&$x` parameter.
    ///
    /// A by-reference parameter is a contract between a call site and a
    /// declaration, and a closure's type is `callable` — ADR 0031 § 4 keeps it
    /// opaque, carrying no parameter list at all, so no call site can know to
    /// stage a cell. The closure may also outlive every frame in scope where
    /// it was written.
    pub const E_CLOSURE_BY_REF_PARAM: Code = Code::new("E0493");
    /// An ADR 0036 § 2 object literal writing one field name twice —
    /// `{a: 1, a: 2}`.
    ///
    /// A shape's fields are a set: the type `{a: int}` names one slot `a`,
    /// and there is no layout under which a literal's two `a`s are both
    /// reachable. PHP's nearest neighbour is a duplicate *array* key, where
    /// the last write wins silently, but an array is a map and a shape is a
    /// record — and the two sides here would not even agree on which write
    /// survives: the interned shape reads the first of the pair while a
    /// class carries one slot per name. Refusing where it is written is the
    /// only answer that keeps both readings out of the language.
    pub const E_DUPLICATE_SHAPE_FIELD: Code = Code::new("E0494");
    /// `->` on a receiver whose declared type can never hold an object —
    /// `int $i = 5; echo $i->name;` — or on one that names no single class,
    /// such as a union of two.
    ///
    /// PHP warns and yields `null` here; ADR 0007 § 7 row 13 makes it a
    /// check-time error instead, for row 8's reason: a declared type is what
    /// makes the answer knowable before the program runs, and nothing in MWL
    /// makes an absent thing read as a zero value. `mixed` is the one receiver
    /// that keeps PHP's *timing* — ADR 0007 § 2's one unchecked position, so
    /// it defers to ADR 0036 § 4's name-keyed fetch and its catchable throw.
    pub const E_RECEIVER_HAS_NO_PROPERTIES: Code = Code::new("E0495");
    /// The right-hand side of `instanceof` naming no class or interface this
    /// program declares — the dynamic form `$x instanceof $name`, a `Core`
    /// class, or an enum.
    ///
    /// The dynamic form is ADR 0007 § 2's rule: a class name is written, never
    /// computed, which is the same line `$$var` and `eval` are already on. A
    /// `Core` class has no descriptor for the test to point at, `Core` classes
    /// being registry signatures rather than declared classes until M7/M8. An
    /// enum is a value type (ADR 0010) and no value of one is ever an object,
    /// so the test has nothing to walk. A written name that resolves to
    /// *nothing* is not here: that is the ordinary `E0303`, exactly as
    /// `new Undeclared()` already reports it.
    pub const E_INSTANCEOF_NOT_A_CLASS: Code = Code::new("E0496");
    /// `instanceof` over a left-hand side whose declared type can hold no
    /// object at all — `int $n = 1; $n instanceof Box;`.
    ///
    /// PHP answers `false`, having no declaration to read; ADR 0007 § 7 row 14
    /// refuses it instead, for the reason ADR 0090 refuses two statically
    /// disjoint types under `==` — the declaration already answered, so the
    /// test is dead code that reads as a live question. `mixed`, `object`, a
    /// shape, a class and any union holding one all keep the run-time test.
    pub const E_INSTANCEOF_SUBJECT_NOT_OBJECT: Code = Code::new("E0497");
    /// An `isset(...)` operand that names no storage — `isset(f())`,
    /// `isset($a + 1)`, `isset(Foo::BAR)`.
    ///
    /// PHP refuses the identical shape at compile time, and its own message
    /// names the replacement: *"Cannot use isset() on the result of an
    /// expression (you can use `null !== expression` instead)"*. ADR 0028 § 3
    /// fixes `isset($x)` as `$x != null`, so an operand that is already a
    /// value rather than a place has nothing `isset` can ask that `!= null`
    /// does not ask more plainly.
    ///
    /// The accepted set is PHP's: a variable, a subscript, a property (`?->`
    /// included), a static property, and any of those in parentheses.
    pub const E_ISSET_NOT_A_VARIABLE: Code = Code::new("E0498");
    /// `static::$prop` — late static binding on a *static property*, whose
    /// storage MWL resolves at compile time.
    ///
    /// PHP re-resolves the name against the **called** class, so a subclass
    /// that redeclares the property gets its own storage through this
    /// spelling and the parent's through `self::`. MWL's slot is fixed where
    /// the access is written (`mwl_ir::ir::StaticProp`), which answers the
    /// same as PHP for every class that does *not* redeclare and differs
    /// silently for one that does — so the spelling is refused rather than
    /// left to diverge, per `AGENTS.md`'s priority 2. `self::$prop` and a
    /// written class name both say exactly which storage is meant and are
    /// unaffected.
    ///
    /// **The last code in the E04xx band.** The next type diagnostic needs a
    /// band decision, not a number.
    pub const E_STATIC_PROPERTY_LATE_BOUND: Code = Code::new("E0499");

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
