//! Source positions, source maps and diagnostics for Novis.
//!
//! Every later stage of the compiler depends on this crate and nothing else
//! depends on those stages, which keeps the dependency graph a line rather than
//! a web.
//!
//! ```
//! use nvs_diagnostics::{code, Diagnostic, Diagnostics, Renderer, SourceMap, Span};
//!
//! let mut map = SourceMap::new();
//! let file = map.add("greet.nvs", "<?nvs\necho $undefined;\n");
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
pub mod embedded;
mod render;
mod source;
mod span;

pub use diagnostic::{Code, Diagnostic, Diagnostics, Label, LabelStyle, Severity, Suggestion};
pub use render::Renderer;
pub use source::{MAX_SOURCE_LEN, PositionEncoding, SourceFile, SourceMap, canonical_key};
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
/// | `E07xx` | types, continued — the `E04xx` band filled at `E0499` |
/// | `E08xx` | types, continued again — the `E07xx` band filled at `E0799` |
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
    /// `rule:types/duration-literal`'s grammar —
    /// out of order, a repeated unit, a fractional count, or longer than
    /// `Core\Time\Duration` can hold. A unit written in the wrong case is a
    /// spelling rather than a shape, and is `E_RESERVED_SPELLING_CASE`.
    pub const E_BAD_DURATION_LITERAL: Code = Code::new("E0007");
    /// A bidirectional control that opens a directional scope and never closes
    /// it inside the source span that opened it, per
    /// `rule:security/bidi-boundaries`
    /// — a comment, a string literal or an inline-HTML run, and each line
    /// of a multi-line one. There is no suppression.
    pub const E_UNBALANCED_BIDI: Code = Code::new("E0008");
    /// An `<?nvs` open tag in a file that opens with `#!` and is therefore
    /// already in code mode, before any `?>` has left it, per
    /// `rule:tooling/shebang-opens-code-mode`
    /// . Reported by `nvs_syntax`'s lexer, which consumes the tag and keeps
    /// lexing code rather than leaving `<` `?` `nvs` for the parser.
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
    /// `rule:security/tainted-qualifier`.
    pub const E_TAINTED_NON_SCALAR: Code = Code::new("E0109");
    /// A class/interface/trait/enum/enum-case/namespace-segment name is not
    /// `PascalCase` — `rule:core-api/identifier-casing`'s casing table.
    pub const E_BAD_TYPE_CASING: Code = Code::new("E0110");
    /// A method name is not `camelCase` — `rule:core-api/identifier-casing`'s casing table. Distinct
    /// from `E_LEGACY_CONSTRUCTOR_SPELLING`, which covers the one mis-cased
    /// spelling (`__construct`) that gets a targeted fix instead of this
    /// generic diagnostic.
    pub const E_BAD_METHOD_CASING: Code = Code::new("E0111");
    /// A property, parameter, local variable or closure self-name is not
    /// `camelCase` — `rule:core-api/identifier-casing`'s casing table, tightened by
    /// `rule:classes/no-leading-underscore-identifiers`, which allows no leading
    /// underscore at all.
    pub const E_BAD_MEMBER_CASING: Code = Code::new("E0112");
    /// A class constant name is not `SCREAMING_SNAKE_CASE` — `rule:core-api/identifier-casing`'s
    /// casing table.
    pub const E_BAD_CONST_CASING: Code = Code::new("E0113");
    /// A method literally named `__construct` —
    /// [ADR 0030](/docs/decisions/0030.md)
    /// §§ 2-3: Novis's constructor is spelled `constructor`, an ordinary
    /// `camelCase` method name needing no exception of its own.
    pub const E_LEGACY_CONSTRUCTOR_SPELLING: Code = Code::new("E0114");
    /// `secret` applied to anything other than `string`/`bytes` — the
    /// qualifier's grammar restricts it to those two scalars, the same
    /// restriction `E_TAINTED_NON_SCALAR` enforces for `tainted`; see
    /// `rule:security/secret-qualifier`.
    pub const E_SECRET_NON_SCALAR: Code = Code::new("E0115");
    /// `tainted secret string`/`tainted secret bytes`: `secret` and `tainted`
    /// compose, but only in the order `secret` before `tainted` — see
    /// `rule:security/secret-qualifier`.
    pub const E_SECRET_TAINTED_ORDER: Code = Code::new("E0116");
    /// A `{name: value, ...}` object literal written where `{` already
    /// commits to a block — an expression-bodied `fn() => {...}`, or a bare
    /// statement-initial `{...}` — needs the same parenthesize-to-force-
    /// expression fix JavaScript uses for the identical ambiguity; see
    /// `rule:types/object-literal`.
    pub const E_OBJECT_LITERAL_NEEDS_PARENS: Code = Code::new("E0117");
    /// `{x}` — an object literal has no shorthand; every field is written
    /// `name: value`. See `rule:types/object-literal`.
    pub const E_OBJECT_LITERAL_SHORTHAND: Code = Code::new("E0118");
    /// `{[$expr]: value}` — an object literal has no computed/dynamic key;
    /// every field name is a static identifier. See `rule:types/object-literal`.
    pub const E_OBJECT_LITERAL_COMPUTED_KEY: Code = Code::new("E0119");
    /// A `float` literal in type position — `rule:types/literal-types` defers float literal
    /// types until floating-point equality has a real answer, so `0.1` names
    /// no type the way `1` and `"a"` do.
    pub const E_FLOAT_LITERAL_TYPE: Code = Code::new("E0120");
    /// An interpolated string in type position — `"a"` is `rule:types/literal-types`'s
    /// singleton type, and a type has no scope to interpolate a variable
    /// from.
    pub const E_INTERPOLATION_IN_TYPE: Code = Code::new("E0121");
    /// A member declaration — property, class constant or method, in a
    /// `class`, `interface` or anonymous-class body — carrying no
    /// `public`/`protected`/`private`, or PHP 8.4's `(set)` form written
    /// without its read visibility. There is no implicit `public`; see
    /// `rule:core-api/written-visibility`.
    /// A class body's PHP `var $x;` reports this too, rather than a message
    /// about the statement grammar it would otherwise fall into (§ 4).
    pub const E_MISSING_VISIBILITY: Code = Code::new("E0122");
    /// An `autoload` prefix, root or glob written as anything but a plain
    /// string literal — an interpolated `"$dir"`, a concatenation, a
    /// variable. Every path resolves at compile time, relative to the file
    /// the declaration appears in, so there is nothing to interpolate from;
    /// see `rule:programs/autoload`, which carries `require`'s literal-only restriction for the same
    /// reason.
    pub const E_AUTOLOAD_PATH_NOT_LITERAL: Code = Code::new("E0123");
    /// A `for` init clause holding a declaration *and* an expression, in
    /// either order — `for (int $i = 0, $j = 1; …)` and
    /// `for ($j = 1, int $i = 0; …)` alike. See
    /// `rule:iteration/for-init-refusals`, whose § 1 makes the clause one or the other and never both.
    pub const E_FOR_INIT_MIXES_DECL_AND_EXPR: Code = Code::new("E0124");
    /// A `for` init clause holding two declarations, `for (int $i = 0, int
    /// $j = 0; …)` — the shape a reader coming from C writes. Separate from
    /// [`E_FOR_INIT_MIXES_DECL_AND_EXPR`] because the fix is different:
    /// the second declaration goes above the loop. `rule:iteration/for-init-refusals`.
    pub const E_FOR_INIT_TWO_DECLARATIONS: Code = Code::new("E0125");
    /// `return`, `break` or `continue` as the body of an expression-level
    /// `catch` arm. See
    /// `rule:expressions/catch-arm-is-an-expression`: an arm holds an expression, which admits `throw` — already an
    /// expression — and refuses the three that are statements. Named rather
    /// than left to the generic expected-expression error, because the fix is
    /// a different construct and not a different token.
    pub const E_CATCH_ARM_NOT_AN_EXPRESSION: Code = Code::new("E0126");
    /// A `///` run with no declaration under it —
    /// `rule:tooling/doc-comment-attaches-to-the-next-declaration`. Reported
    /// rather than ignored because the marker is what separates documentation
    /// from a note to self: a `///` that documents nothing is either a note
    /// written with the wrong marker or a declaration that got deleted out from
    /// under it, and both are worth saying. The fix is one slash fewer.
    pub const E_DOC_COMMENT_UNATTACHED: Code = Code::new("E0127");
    /// An `@tag` at the start of a doc comment line that is neither `@see` nor
    /// `@example` — `rule:tooling/doc-comment-tags-are-see-and-example`. One
    /// code for every rejected spelling, with the help naming what to write
    /// instead, because the answer for `@param` and the answer for an invented
    /// tag differ in wording and not in kind: the set is closed, and this
    /// diagnostic is the entire difference between a closed set and a
    /// convention.
    pub const E_DOC_COMMENT_UNKNOWN_TAG: Code = Code::new("E0128");
    /// The right side of a `|>` with no `$_` in it — the help names the shape
    /// (`Str::trim($_)`), and when the right side is first-class callable
    /// syntax or a closure value it adds that PHP 8.5's `|>` applies a
    /// callable where this one substitutes a hole.
    /// `rule:expressions/pipeline-hole-once`: the whole affordability of
    /// sharing the spelling with PHP is that the habit is refused at the
    /// character where it goes wrong rather than meaning something else.
    pub const E_PIPELINE_RIGHT_SIDE_HAS_NO_HOLE: Code = Code::new("E0129");
    /// `$_` more than once on one right side of a `|>`, whose help is to bind
    /// the value to a local instead. Exactly one hole is what makes
    /// `rule:expressions/pipeline-substitution` a substitution with no
    /// temporary and no double evaluation, so this is a refusal and not a
    /// second lowering.
    pub const E_PIPELINE_RIGHT_SIDE_REPEATS_THE_HOLE: Code = Code::new("E0130");
    /// `$_` anywhere that is not the right side of a `|>`, where it names
    /// nothing — `rule:expressions/pipeline-hole-once`. Separate from the
    /// unresolved-variable error because the fix is a `|>` and not a
    /// declaration.
    pub const E_HOLE_OUTSIDE_A_PIPELINE: Code = Code::new("E0131");
    /// `tainted {…}` over a shape carrying no `string` and no `bytes` anywhere
    /// — `rule:security/tainted-qualifier`. The qualifier distributes to text
    /// and there is none to reach, so it promises something nothing enforces,
    /// which is worth a diagnostic rather than the silent no-op it would
    /// otherwise be.
    pub const E_TAINTED_SHAPE_HAS_NO_TEXT: Code = Code::new("E0132");
    /// A modifier run or an attribute group written in front of a body's
    /// `type` alias — `rule:types/type-alias`. An alias is reachable wherever
    /// its owner's name is, so there is no visibility to write, and nothing
    /// downstream of the checker ever sees the name, so an attribute has
    /// nothing to attach to. Parsing the run and dropping it would accept a
    /// declaration saying something the language does not have.
    pub const E_TYPE_ALIAS_TAKES_NO_MODIFIER_OR_ATTRIBUTE: Code = Code::new("E0133");

    // --- E02xx rejected PHP constructs -------------------------------------
    // Novis accepts PHP 8.5 syntax as a *pragmatic* superset. These constructs
    // are recognised — so the diagnostic can be precise and suggest a
    // replacement — and are then rejected. Most parse first; the ones a
    // *lexical* rule refuses (`E_RESERVED_SPELLING_CASE`,
    // `E_IDENTITY_OPERATOR_UNSUPPORTED`, `E_ANGLE_NOT_EQUAL_UNSUPPORTED`) are
    // named where they are recognised, which is the lexer. A few are shapes
    // PHP refuses as well and admits here by nothing but omission; they are in
    // this band because the answer is the band's own — recognise the shape and
    // name its rewrite — and because docs/adr/README.md § *Decisions taken at
    // project start* places them here. See docs/spec.
    /// `eval()`: Novis compiles ahead of execution.
    pub const E_EVAL_UNSUPPORTED: Code = Code::new("E0201");
    /// `$$name` and `${$name}`: defeats name resolution and type inference.
    pub const E_VARIABLE_VARIABLE: Code = Code::new("E0202");
    /// `goto`: makes the control-flow graph unstructured.
    pub const E_GOTO_UNSUPPORTED: Code = Code::new("E0203");
    /// `global $x`: use a parameter or an explicit process-scoped binding.
    pub const E_GLOBAL_UNSUPPORTED: Code = Code::new("E0204");
    /// `extract()`: introduces bindings whose names are not known statically.
    pub const E_EXTRACT_UNSUPPORTED: Code = Code::new("E0205");
    /// A PHP C extension that has no Novis equivalent.
    pub const E_UNSUPPORTED_EXTENSION: Code = Code::new("E0206");
    /// A `preg` pattern using a construct the pure-Rust engine cannot express.
    pub const E_UNSUPPORTED_REGEX: Code = Code::new("E0207");
    /// `settype($x, ...)`: no assignment, operator or call can change what a
    /// binding's type is.
    pub const E_SETTYPE_UNSUPPORTED: Code = Code::new("E0208");
    /// `static $x = ...;` inside a function: there is no function-scope
    /// storage class — see `rule:statements/static-is-a-member-modifier`.
    pub const E_STATIC_LOCAL_UNSUPPORTED: Code = Code::new("E0209");
    /// `static function`/`static fn`: closures already capture `$this` only
    /// if they use it, so `static` has nothing left to mean here.
    pub const E_STATIC_CLOSURE_UNSUPPORTED: Code = Code::new("E0210");
    /// A PHP superglobal (`$_GET`, `$_SERVER`, `$GLOBALS`, `$argv`, …): no
    /// variable is ever populated by the host — see `rule:statements/no-host-populated-variables`.
    pub const E_SUPERGLOBAL_UNSUPPORTED: Code = Code::new("E0211");
    /// `use Path\To\Name as Other;`: an import cannot be renamed — see
    /// `rule:statements/nothing-gets-a-second-name`.
    pub const E_IMPORT_ALIAS_UNSUPPORTED: Code = Code::new("E0212");
    /// `function foo() { ... }` outside any class: a function must be a
    /// method — see `rule:classes/no-free-functions-or-constants`.
    pub const E_TOPLEVEL_FUNCTION_UNSUPPORTED: Code = Code::new("E0215");
    /// `const FOO = 1;` outside any class: a constant must belong to a
    /// class — see `rule:classes/no-free-functions-or-constants`.
    pub const E_TOPLEVEL_CONST_UNSUPPORTED: Code = Code::new("E0216");
    /// `namespace Core;` (or anything nested under it) in user source:
    /// `Core` is reserved for built-ins — see `rule:core-api/reserved-namespace`.
    pub const E_RESERVED_CORE_NAMESPACE: Code = Code::new("E0217");
    /// `enum Name implements Iface { ... }`: an enum declares only cases and
    /// an optional backing type — see `rule:enums/no-class-machinery`.
    pub const E_ENUM_IMPLEMENTS_UNSUPPORTED: Code = Code::new("E0218");
    /// `enum Name: string { ... }`: no `string` backing, only `int`/`uint` —
    /// see `rule:enums/no-class-machinery`.
    pub const E_ENUM_STRING_BACKING_UNSUPPORTED: Code = Code::new("E0219");
    /// A method, property, class constant or trait use inside an `enum`
    /// body: an enum declares only cases and an optional backing type — see
    /// `rule:enums/no-class-machinery`.
    pub const E_ENUM_MEMBER_UNSUPPORTED: Code = Code::new("E0220");
    /// `include`, `include_once`, or `require_once`: Novis keeps exactly one
    /// same-frame inclusion construct, `require` — see `rule:statements/require-is-the-only-inclusion-construct`.
    pub const E_INCLUDE_FAMILY_UNSUPPORTED: Code = Code::new("E0221");
    /// An anonymous `function (...) { ... }` literal, with or without a
    /// `use` clause: `fn` is the only closure literal — see `rule:types/closure-literal`.
    pub const E_FUNCTION_CLOSURE_UNSUPPORTED: Code = Code::new("E0222");
    /// `use ($y)` on a closure literal: capture is always implicit and by
    /// value, so there is no clause to write — see `rule:types/implicit-capture`.
    pub const E_CLOSURE_USE_UNSUPPORTED: Code = Code::new("E0223");
    /// `use (&$y)` on a closure literal specifically: by-reference capture
    /// has no replacement syntax — see `rule:types/implicit-capture`.
    pub const E_CLOSURE_USE_BY_REF_UNSUPPORTED: Code = Code::new("E0224");
    /// PHP's legacy `(T)expr` cast syntax — `as` is the only conversion
    /// spelling. See `rule:types/no-legacy-cast`, which amends `rule:types/conversion`.
    pub const E_LEGACY_CAST_UNSUPPORTED: Code = Code::new("E0225");
    /// PHP's `and`/`or`/`xor` keyword operators — `&&`/`||` are the only
    /// logical connectives. See `rule:expressions/no-keyword-logical-operators`.
    pub const E_LOGICAL_KEYWORD_UNSUPPORTED: Code = Code::new("E0226");
    /// `trait Name { … }`, `use TraitName, ...;` inside a class body, or
    /// `insteadof` anywhere: traits do not exist — an interface
    /// default/private method replaces shared behavior, and
    /// `implements Interface by $field;` replaces shared state. See
    /// `rule:classes/no-traits`. One code for the whole shape: there is no
    /// trait `use { ... }` adaptation grammar to diagnose more finely, since
    /// traits do not exist at all. The default-method/delegation conflict is
    /// `E_INTERFACE_MEMBER_CONFLICT` (also `rule:classes/no-traits`), which is
    /// `nvs-hir`'s resolution work rather than this diagnostic's.
    pub const E_TRAIT_NOT_SUPPORTED: Code = Code::new("E0227");
    /// `die`, in any position `exit` is also accepted: Novis keeps exactly one
    /// process-termination keyword. See `rule:statements/exit-is-the-only-termination-keyword`.
    pub const E_DIE_UNSUPPORTED: Code = Code::new("E0228");
    /// The `<?php` open tag: Novis keeps exactly one code-mode open tag,
    /// `<?nvs` (plus the short-echo `<?=`). See `rule:statements/nvs-is-the-only-open-tag`.
    pub const E_PHP_OPEN_TAG_UNSUPPORTED: Code = Code::new("E0229");
    /// `list(...)` as a destructuring target: Novis keeps exactly one
    /// destructuring spelling, `[...]`. See `rule:expressions/bracket-destructuring`.
    pub const E_LIST_DESTRUCTURING_UNSUPPORTED: Code = Code::new("E0230");
    /// A reserved lexical spelling written in anything but lower case —
    /// A reserved spelling written in a case Novis does not have: `<?NVS`
    /// rather than `<?nvs`, and a duration literal's unit, `30S` rather than
    /// `30s`. PHP matches its reserved spellings case-insensitively; Novis
    /// accepts exactly one spelling of each, so a program's meaning never
    /// depends on the case a reserved word was typed in. See
    /// `rule:classes/reserved-spellings-are-lower-case`. That is what puts a
    /// Novis-only spelling in this band: the class the code names is the
    /// case-insensitivity PHP has and Novis does not, of which the open tag is
    /// the PHP-visible half. The primary span is the spelling itself, which is
    /// what `rule:tooling/fmt-normalizes-only-reserved-spellings` lower-cases.
    /// A mis-cased *keyword* (`IF`, `TRUE`) gets no diagnostic of its
    /// own — it is simply an ordinary identifier, since `rule:core-api/identifier-casing` makes
    /// `IF` a legal class name the lexer cannot tell apart from a mis-typed
    /// `if`.
    pub const E_RESERVED_SPELLING_CASE: Code = Code::new("E0231");
    /// PHP's `===`/`!==` — Novis keeps exactly one equality operator, `==`, and
    /// its negation `!=`. See
    /// `rule:expressions/one-equality-operator`. This is the one construct in this band the *lexer* reports rather
    /// than the parser: there is nothing to parse precisely here, so the
    /// three characters are consumed, named, and lexed as the two-character
    /// operator so the rest of the file still reports its own problems.
    pub const E_IDENTITY_OPERATOR_UNSUPPORTED: Code = Code::new("E0232");
    /// A `class`, `interface` or `enum` declaration written inside a function
    /// body, a property hook or a nested block. PHP declares such a type when
    /// the statement *runs*, so whether the name exists depends on control
    /// flow; Novis resolves every type name against a static table built before
    /// any code runs. See `docs/adr/README.md`
    /// § *Decisions taken at project start*, and `nvs_types::locals`' module
    /// doc for the walk that reports it.
    pub const E_NESTED_TYPE_DECLARATION_UNSUPPORTED: Code = Code::new("E0233");
    /// An `unset()` operand that is not an array element of a named holder —
    /// `unset($x)` on a bare local, or a subscript of a temporary such as
    /// `unset(rows()["k"])`. `rule:classes/unset-is-refused-on-a-property` keeps `unset()` for exactly one job,
    /// removing an array entry: a binding is declared with a type and
    /// definitely assigned (`rule:types/declaration`), so there is no "undefined again"
    /// state for a local to return to, and a temporary has nothing for ADR
    /// 0007 § 5's separated array to be written back into. An operand that is
    /// a *declared* property is [`E_UNSET_ON_PROPERTY`] instead, which owns
    /// that half of the same section.
    pub const E_UNSET_TARGET_NOT_AN_ELEMENT: Code = Code::new("E0234");
    /// A member name that is computed rather than written out — `$obj->$name`,
    /// `$obj->{$expr}`, and the same two spellings in front of a call's
    /// parentheses. The sibling of [`E_VARIABLE_VARIABLE`] one level in: a
    /// name only known when the statement runs defeats the resolution every
    /// property access below the checker is built on, and it is the one
    /// spelling that would let a request-controlled string pick which field
    /// to read or write.
    ///
    /// `rule:classes/no-dynamic-properties` keeps its runtime-throw half for the two ways a name
    /// genuinely arrives late — a reflection-based get/set, and `rule:types/erased-member-access`'s
    /// erased receiver, where the name *is* written out and only the class
    /// behind the handle is unknown. Neither needs this spelling, and `rule:types/object-literal` already refuses its literal-side twin, the computed shape key
    /// `{[$expr]: 1}`.
    ///
    /// `rule:types/property-key-access` carves out the one exception and moves the report with it:
    /// `$obj->$key` is admitted where `$key`'s type is a `property<T>` the
    /// receiver satisfies — a set of names checked where the `as` was written,
    /// so nothing request-controlled picks a field — and every other operand is
    /// still this code, reported by `nvs_types` rather than by the parser,
    /// since the operand's type is what decides and a parser sees none.
    pub const E_DYNAMIC_MEMBER_NAME: Code = Code::new("E0235");
    /// `@expr` — PHP's error-suppression prefix. There is nothing for it to
    /// suppress: `rule:errors/escalation-ladder`
    /// makes every runtime failure a `Throwable` propagated by checked return
    /// (`rule:errors/propagation`), not a
    /// diagnostic printed alongside a value, and `rule:core-api/removals` already lists
    /// `@` among the constructs that decision closes. `try`/`catch` is the
    /// replacement, and it is the only one.
    pub const E_SUPPRESSION_UNSUPPORTED: Code = Code::new("E0236");
    /// `&` written where PHP puts a by-reference marker — a parameter
    /// (`f(int &$x)`), a `foreach` value binding (`foreach ($xs as int &$v)`),
    /// a destructuring leaf (`[int &$a] = $pair`), a by-reference return
    /// (`function &f()`) or a by-reference property hook (`&get`).
    ///
    /// `rule:statements/inout-is-the-by-reference-spelling`
    /// retires `&` as a by-reference marker: the two binding modes it spelled
    /// are written `inout`, before the type and again at the call site, and
    /// the two *returning* forms have no replacement at all — Novis hands back
    /// a value, never a place. `&` keeps its other jobs unchanged, so `$a &
    /// $b` is still bitwise AND and `A&B` is still an intersection type.
    ///
    /// The spellings refused because the language has no such thing keep
    /// their own codes and are deliberately not this one:
    /// [`E_ASSIGN_BY_REFERENCE`], [`E_ARRAY_ELEMENT_BY_REFERENCE`] and
    /// [`E_CLOSURE_USE_BY_REF_UNSUPPORTED`] each name a rule rather than a
    /// spelling, and `inout` is not what replaces any of them.
    pub const E_BY_REFERENCE_MARKER_RETIRED: Code = Code::new("E0237");
    /// `use App\Models\{User, Post};` — PHP's group-use form. One `use`
    /// statement imports exactly one name, so the short name a file introduces
    /// is always written on a line of its own: `docs/adr/README.md`
    /// § *Decisions taken at project start* owns the rule, and
    /// [`E_IMPORT_ALIAS_UNSUPPORTED`] is its sibling refusal on the other half
    /// of the same statement.
    pub const E_IMPORT_GROUP_UNSUPPORTED: Code = Code::new("E0238");
    /// PHP's `case Name = 1;` enum-body spelling. Novis writes a case as a
    /// bare `Name = 1,` in a comma list, with no `case` keyword — see
    /// `rule:enums/declaration`. Raised on the `case` keyword itself, once,
    /// rather than as an [`E_ENUM_MEMBER_UNSUPPORTED`] cascade: that code is
    /// for a *member* in an enum body and its "move this to a separate class"
    /// help is the wrong answer here, since the case belongs in the enum and
    /// only its spelling is wrong.
    pub const E_PHP_ENUM_CASE_UNSUPPORTED: Code = Code::new("E0239");
    /// A leading `\` on a name — `\App\Models\User`, `use \App\Models\User;`,
    /// `namespace \App;`. A name containing a `\` is already read from the
    /// root, so the prefix has no work left to do, and accepting it would be
    /// the second spelling
    /// `rule:statements/a-leading-separator-does-not-parse`
    /// exists to remove. PHP rejects the `namespace` spelling too; the other
    /// two it accepts, which is what made the token mean three different
    /// things by position.
    pub const E_LEADING_BACKSLASH_UNSUPPORTED: Code = Code::new("E0240");
    /// `<>` for inequality, PHP's inherited second spelling of `!=`.
    /// `rule:expressions/one-equality-operator` makes
    /// `==` and `!=` the whole set, so this is the same decision
    /// [`E_IDENTITY_OPERATOR_UNSUPPORTED`] reports and not a lexical accident;
    /// it is separate from that code because the fix is a different edit and
    /// the reason is spelling rather than semantics — `<>` means exactly what
    /// `!=` means.
    pub const E_ANGLE_NOT_EQUAL_UNSUPPORTED: Code = Code::new("E0241");
    /// A `try` block with neither a `catch` clause nor a `finally` — `try { …
    /// }` alone, which guards nothing and is the shape a deleted clause leaves
    /// behind. PHP refuses it too. See `docs/adr/README.md` § *Decisions taken
    /// at project start*.
    pub const E_TRY_WITHOUT_CLAUSE: Code = Code::new("E0242");
    /// The braced `namespace X { … }` form, and with it a file holding two
    /// namespaces. `rule:security/authority-is-the-enclosing-namespace`
    /// keys authority on the enclosing namespace, so a file that is two
    /// namespaces is a file whose authority is a function of the line number.
    /// The rewrite is one file per namespace, declared `namespace X;`.
    pub const E_BRACED_NAMESPACE_UNSUPPORTED: Code = Code::new("E0243");
    /// `new class { … }`: an anonymous class is a nested declaration in
    /// expression position, with no name for the static class table to hold —
    /// the same refusal a conditionally declared class gets, for the same
    /// reason. The rewrite is a named class in the same file, or a closure
    /// (`rule:types/closure-literal`).
    pub const E_ANONYMOUS_CLASS_UNSUPPORTED: Code = Code::new("E0244");
    /// `catch (A | B $e)`, PHP's multi-class clause. The binding carries one
    /// static type (`rule:types/declaration`'s `catch` row), so a clause naming two classes has no type to give
    /// it; the rewrite is one clause per class, each with its own variable
    /// name, or one clause naming a class they all extend. The expression form
    /// refuses the same shape with the same words
    /// (`rule:expressions/catch-expression`).
    pub const E_CATCH_UNION_TYPE_UNSUPPORTED: Code = Code::new("E0245");
    /// `public const LIMIT = 9;`, PHP's untyped class constant. Every other
    /// binding in the language writes its type
    /// (`rule:types/declaration`) and a
    /// constant is no different: the type read off the folded value is a
    /// guess the declaration never made, it is unavailable to an interface
    /// constant at all, and where the value has no constant form there is
    /// nothing to guess from. See `docs/adr/README.md` § *Decisions taken at
    /// project start*.
    pub const E_CONSTANT_WITHOUT_TYPE: Code = Code::new("E0246");
    /// `let` or `is` written as a name, which PHP 8.6 deprecates and Novis
    /// refuses outright (`rule:php-migration/let-and-is-are-reserved`). Both
    /// are reserved for a construct that does not exist — the family `eval`,
    /// `goto` and `list` are already in — so the spelling stays available and
    /// the help names the living one: `var` declares an inferred local,
    /// `instanceof` tests and `as` converts. The rewrite is a rename.
    pub const E_RESERVED_FOR_FUTURE_USE: Code = Code::new("E0247");
    /// `return $value;` inside a `constructor`, which PHP 8.6 deprecates and
    /// this refuses (`rule:php-migration/a-constructor-return-carries-no-value`).
    /// The object under construction is the result and nothing else can be. A
    /// bare `return;` still leaves early and
    /// `rule:classes/definite-property-initialization` goes on checking that
    /// path, so only the value is named; the rewrite is dropping it.
    pub const E_CONSTRUCTOR_RETURN_CARRIES_A_VALUE: Code = Code::new("E0248");
    /// `public readonly int $n = 1;`, which PHP 8.6 allows and this refuses
    /// (`rule:php-migration/a-readonly-property-declares-no-default`).
    /// `readonly` is one assignment during construction, so a property whose
    /// single assignment is its own declaration-site default is a per-instance
    /// constant — and `const` already spells one (`rule:types/class-constant`).
    /// The help names moving the value into the constructor and dropping
    /// `readonly` as the other fix.
    pub const E_READONLY_PROPERTY_WITH_DEFAULT: Code = Code::new("E0249");
    /// A `return` written inside a `finally` block, which PHP 8.6 deprecates
    /// for removal and this refuses
    /// (`rule:php-migration/no-return-leaves-a-finally`). It replaces whatever
    /// the region was leaving with, including a throw in flight — the one
    /// construct where an unhandled exception vanishes with no handler
    /// anywhere. The help names the two rewrites the rule leaves: change the
    /// result in a `catch`, or write it after the region.
    pub const E_RETURN_LEAVES_A_FINALLY: Code = Code::new("E0250");
    /// A `break` or `continue` inside a `finally` block whose level reaches
    /// past the loops and `switch`es the block itself opened
    /// (`rule:php-migration/no-return-leaves-a-finally`). Leaving the block
    /// discards what the region was leaving with, which is
    /// [`E_RETURN_LEAVES_A_FINALLY`]'s objection; a loop written wholly inside
    /// the block keeps both spellings, so only the reaching level is named.
    pub const E_BREAK_LEAVES_A_FINALLY: Code = Code::new("E0251");

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
    /// `rule:types/alias-is-never-a-bare-class`.
    pub const E_TYPE_ALIAS_ALIASES_CLASS: Code = Code::new("E0307");
    /// A `Class::member` reference (a static call, a class constant, an
    /// enum case, or a static property) names nothing declared on that class
    /// or any of its `extends`/`implements` ancestors — `rule:classes/no-free-functions-or-constants`'s "every
    /// callable and constant is a class member" has no bare-name fallback to
    /// fall into instead.
    pub const E_UNDEFINED_MEMBER: Code = Code::new("E0309");
    /// A `type` alias whose expansion, followed far enough, refers back to
    /// itself — `type A = B; type B = A;` or any longer cycle; see
    /// `rule:types/type-alias`.
    pub const E_TYPE_ALIAS_CYCLE: Code = Code::new("E0310");
    /// A `require` whose path is a literal, resolved statically per
    /// `rule:statements/require-is-the-only-inclusion-construct`, but does not name a file that can be loaded as source (it
    /// does not exist, or is not valid UTF-8).
    pub const E_REQUIRE_TARGET_NOT_FOUND: Code = Code::new("E0311");
    /// A `require` chain whose statically-resolved literal paths lead back
    /// to a file already being resolved — `require`'s own semantics (same
    /// frame, runs every time reached) give this no other resolution than a
    /// diagnostic, the same way `crate::hierarchy`'s `extends`/trait-use
    /// cycle and `crate::aliases`'s `type` alias cycle are both handled.
    pub const E_CIRCULAR_REQUIRE: Code = Code::new("E0312");
    /// `$this->name` where `name` is not declared on the enclosing class or
    /// any `extends`/`implements`/trait-use ancestor — `rule:classes/no-dynamic-properties`'s "no
    /// `__get`/`__set` fallback" for the one receiver shape resolvable
    /// without a type checker.
    pub const E_UNDEFINED_PROPERTY: Code = Code::new("E0313");
    /// A `require` whose literal path resolves on disk only because the
    /// filesystem is case-insensitive — `require 'mailer.nvs';` finding
    /// `Mailer.nvs`. Reported on Windows/macOS so the same source is not a
    /// `E_REQUIRE_TARGET_NOT_FOUND` on Linux; see
    /// [ADR 0062](/docs/decisions/0062.md)
    /// § 3, which extends
    /// `rule:programs/autoload`'s exact-name rule from `autoload` to `require`.
    pub const E_REQUIRE_PATH_CASE_MISMATCH: Code = Code::new("E0314");
    /// Two `autoload` declarations in one program claim the same namespace
    /// prefix — `rule:programs/autoload`'s "one prefix has one home". An explicit prefix deliberately does
    /// *not* collide with a `discover` glob that would produce the same one:
    /// there the glob skips the name, which is what makes a vendor override
    /// work.
    pub const E_DUPLICATE_AUTOLOAD_PREFIX: Code = Code::new("E0315");
    /// An `autoload` declaration in a file that was itself reached through
    /// the autoload map — `rule:programs/autoload`, which honors a declaration only in a
    /// file reachable by `require` from the entry point, since otherwise the
    /// map would depend on itself.
    pub const E_AUTOLOAD_IN_AUTOLOADED_FILE: Code = Code::new("E0316");
    /// A file reached through an autoload root that does not hold exactly one
    /// top-level declaration named after it — `rule:programs/one-declaration-per-autoloaded-file`. Without the rule,
    /// whether a name exists in the program depends on what was resolved
    /// first, which makes the build non-reproducible and the cache unkeyable.
    pub const E_AUTOLOAD_FILE_SHAPE: Code = Code::new("E0317");
    /// An `autoload discover` glob that is not one `*` occupying a whole path
    /// segment, or whose base directory does not exist — `rule:programs/autoload`. A
    /// *matched* directory with an unusable name is skipped in silence (a
    /// glob over a filesystem always sweeps `.git` and `vendor`); the glob
    /// itself is diagnosed, since one that silently discovers nothing is the
    /// worst outcome on offer.
    pub const E_AUTOLOAD_GLOB_SHAPE: Code = Code::new("E0318");
    /// A bare name used where a value is expected — `PHP_EOL`, `MY_LIMIT` —
    /// which in PHP would be a global constant fetch.
    /// [ADR 0011](/docs/decisions/0011.md)
    /// § 3 leaves no such storage row: a constant is always a class
    /// constant, so there is no name for this to resolve against and nothing
    /// below the resolver to lower it to.
    pub const E_NO_GLOBAL_CONSTANT: Code = Code::new("E0319");
    /// A bare name called as a function — `strlen($s)` — which in PHP would
    /// be a global function call. `rule:classes/no-free-functions-or-constants`: every callable is a method,
    /// with no exception for built-ins, which live under the reserved `Core`
    /// namespace. Split from [`E_NO_GLOBAL_CONSTANT`] because the two carry
    /// different replacements even though the callee is the same node.
    pub const E_NO_FREE_FUNCTION: Code = Code::new("E0320");
    /// `self`, `static` or `parent` written where a value is expected, rather
    /// than on the left of a `::`. Each of the three names a *class*, and a
    /// class is not a value in Novis — there is no class-object reflection
    /// handle (`rule:classes/no-free-functions-or-constants`
    /// puts every reflective question on `Core\Reflect` instead).
    pub const E_CLASS_NAME_NOT_A_VALUE: Code = Code::new("E0321");
    /// A qualified name that does not resolve, but which PHP's rule would have
    /// resolved relative to the enclosing namespace — `Models\User` written
    /// inside `namespace App;`, meaning `App\Models\User`. This is the one
    /// construct
    /// `rule:statements/a-qualified-name-is-absolute`
    /// changes the meaning of, so it gets a diagnostic naming the absolute
    /// spelling rather than the generic [`E_UNDEFINED_CLASS`] a reader would
    /// otherwise have to work backwards from. A qualified name that resolves
    /// neither way is that generic error, not this one.
    pub const E_RELATIVE_QUALIFIED_NAME: Code = Code::new("E0322");
    /// A doc comment's `@see` naming something that does not resolve — a class
    /// nothing declares, a member no class in the chain has, or nothing at all
    /// after the tag. `rule:tooling/doc-comment-tags-are-see-and-example`
    /// keeps the tag set at two by making each one buy a check, and this is
    /// `@see`'s: a cross-reference that resolves where it is written cannot
    /// rot into a link to a member that has since been renamed. Resolution
    /// rather than the parser, because the question is what a name means, and
    /// the same one [`E_UNDEFINED_MEMBER`] asks of a `Class::member` in code.
    pub const E_DOC_SEE_UNRESOLVED: Code = Code::new("E0323");
    /// A doc comment's `@example` naming a path that is in a directory the
    /// test corpus walks but holds no file. The other half of
    /// `rule:tooling/doc-comment-tags-are-see-and-example`'s check on the tag
    /// is [`E_DOC_EXAMPLE_NOT_WALKED`], split from this one because the two
    /// carry opposite repairs: write the file, or move it.
    pub const E_DOC_EXAMPLE_NOT_FOUND: Code = Code::new("E0324");
    /// A doc comment's `@example` naming a path in no directory the test
    /// corpus walks. An example nothing compiles is one that rots in a page
    /// while the member it documents moves on, which is the whole reason
    /// `rule:tooling/doc-comment-tags-are-see-and-example` keeps the tag: a
    /// path outside `examples/` and `tests/` buys no check.
    pub const E_DOC_EXAMPLE_NOT_WALKED: Code = Code::new("E0325");
    /// A public member carrying no doc comment, reported only under
    /// `nvs check --strict-docs`. `rule:tooling/strict-docs` keeps it silent
    /// everywhere else and no autofix can satisfy it: with no `@param` and no
    /// `@return` to fill in, a generated `///` would be empty, so the only way
    /// to clear it is to write a sentence.
    pub const E_DOC_MISSING: Code = Code::new("E0326");

    // --- E04xx types -------------------------------------------------------
    /// A value whose type cannot be what this position requires.
    pub const E_TYPE_MISMATCH: Code = Code::new("E0401");
    /// Wrong number of arguments — including a **shape key** a member
    /// requires and a written literal does not carry. `rule:core-api/shape-flattens-at-the-abi` flattens
    /// each key of a shape parameter into one argument of its own, so an
    /// omitted required key is a call one argument short rather than a
    /// separate kind of mistake.
    pub const E_ARITY_MISMATCH: Code = Code::new("E0402");
    /// A return type that no return statement can satisfy.
    pub const E_BAD_RETURN_TYPE: Code = Code::new("E0403");
    /// An override whose signature is not compatible with the parent's.
    ///
    /// One incompatibility is reported under it: a member an ancestor declared
    /// `: static` answered by a declaration that writes a class of its own.
    /// `nvs_types::conformance::reject_dropped_static_return` owns why that one
    /// is not an assignability question, and argument and return assignability
    /// across an override is a rule no decision has reached yet.
    pub const E_INCOMPATIBLE_OVERRIDE: Code = Code::new("E0404");
    /// A property or method access on a type that has no such member.
    pub const E_UNKNOWN_MEMBER: Code = Code::new("E0405");
    /// A local variable declared a second time while its first declaration
    /// is still live — `rule:types/declaration`: "there is no shadowing."
    pub const E_REDECLARED_LOCAL: Code = Code::new("E0406");
    /// `int ⊕ uint` arithmetic — `rule:types/arithmetic`: there is no representable
    /// common type, so one side must be converted explicitly.
    pub const E_INT_UINT_ARITHMETIC: Code = Code::new("E0407");
    /// An `array<...>` type nests past `rule:types/arrays`'s depth-32 bound.
    pub const E_ARRAY_TYPE_TOO_DEEP: Code = Code::new("E0408");
    /// A non-nullable, no-default property a class declares (itself, or
    /// through a used trait) is not definitely assigned on some path out of
    /// its constructor — or the class has no constructor at all to assign
    /// it; see `rule:classes/definite-property-initialization`.
    pub const E_UNINITIALIZED_PROPERTY: Code = Code::new("E0409");
    /// A subclass constructor has a path that never calls
    /// `parent::constructor(...)`, so the properties it inherits are never
    /// discharged on that path; see `rule:classes/definite-property-initialization`.
    pub const E_MISSING_PARENT_CONSTRUCTOR_CALL: Code = Code::new("E0410");
    /// `<`/`>`/`<=`/`>=`/`<=>` between two objects whose static types are not
    /// both provably the same class implementing the reserved global
    /// `Comparable` interface — either one side doesn't implement it, or the
    /// two sides are different classes even though both do; see `rule:classes/comparable` and `rule:classes/comparable-is-same-class-only`. PHP's implicit property-walk fallback has no Novis equivalent.
    pub const E_COMPARISON_REQUIRES_COMPARABLE: Code = Code::new("E0411");
    /// An object used at an implicit string-conversion site (interpolation,
    /// concatenation, `echo`/`print`, `as string`/`(string)`) whose static
    /// type does not provably implement the reserved global `Stringable`
    /// interface; see `rule:classes/stringable`. PHP's own fallback here is already a
    /// fatal error, so nothing permissive is being removed.
    pub const E_STRINGABLE_REQUIRED: Code = Code::new("E0412");
    /// `unset()` on a declared property, static or instance, regardless of
    /// nullability — refused outright because `rule:classes/definite-property-initialization` already guarantees no
    /// declared property is ever anything but definitely initialized; see ADR
    /// 0028 § 3. A static property is the same slot and the same guarantee, so
    /// it takes the same code, named for the class that *declares* it. Every
    /// other operand `unset()` cannot remove an entry from is
    /// [`E_UNSET_TARGET_NOT_AN_ELEMENT`].
    pub const E_UNSET_ON_PROPERTY: Code = Code::new("E0413");
    /// `var $x = [...];` — a bare array literal has no target type to check
    /// against, the one initializer shape `var` cannot infer from; see
    /// `rule:types/var-inference`.
    pub const E_VAR_ARRAY_LITERAL_NEEDS_TYPE: Code = Code::new("E0414");
    /// An arithmetic or bitwise operator applied directly to an enum-typed
    /// operand — neither is defined on an enum type; convert to its
    /// underlying `int`/`uint` with `as` first. See `rule:types/conversion`.
    pub const E_ENUM_ARITHMETIC_UNSUPPORTED: Code = Code::new("E0415");
    /// `as` from one enum type to a *different* enum type, even when both
    /// share the same underlying integer type — rejected outright; an
    /// explicit `match` naming every case is the replacement. See `rule:types/conversion`.
    pub const E_ENUM_CONVERSION_UNSUPPORTED: Code = Code::new("E0416");
    /// `as Core\Html\Markup` on anything but a source-literal string — a
    /// runtime-computed or `tainted` value can never become trusted markup
    /// this way, closing "compute the escape-defeating payload at runtime,
    /// then cast it." See `rule:core-classes/html-auto-escape`.
    pub const E_MARKUP_REQUIRES_LITERAL: Code = Code::new("E0417");
    /// A string passed (or convertible without laundering) where `callable`
    /// is the declared type — PHP's bare-name/`"Class::method"` callable
    /// spellings are both rejected in favor of first-class callable syntax.
    /// See `rule:types/callable-is-a-closure`.
    pub const E_CALLABLE_STRING_UNSUPPORTED: Code = Code::new("E0418");
    /// A `[$obj, 'method']`-shaped array passed where `callable` is the
    /// declared type. See `rule:types/callable-is-a-closure`.
    pub const E_CALLABLE_ARRAY_UNSUPPORTED: Code = Code::new("E0419");
    /// `$obj(...)` where `$obj`'s static type is not `callable` — Novis has no
    /// `__invoke`, so no class ever makes `()` mean anything else. See
    /// `rule:types/callable-is-a-closure`.
    pub const E_NOT_CALLABLE: Code = Code::new("E0420");
    /// A `secret`-qualified value reaching a `Core\Html\Markup`-building
    /// conversion — refused even though the equivalent `tainted`-only value
    /// would (once `Core\Html` exists) be auto-escaped instead: escaping
    /// neutralizes injection risk, not confidentiality. See `rule:security/secret-sinks-refuse`.
    pub const E_SECRET_MARKUP_UNSUPPORTED: Code = Code::new("E0421");
    /// A `secret`-qualified value passed as a `Throwable`-shaped class's
    /// constructor message argument — closing the common leak of a
    /// credential ending up in a stack trace or an error page. See `rule:security/secret-sinks-refuse`
    /// .
    pub const E_SECRET_THROWABLE_MESSAGE: Code = Code::new("E0422");
    /// The `parent` type atom (`parent $x`, a parameter/property/return
    /// position — distinct from `new parent(...)`, which silently falls back
    /// to `mixed` for the same shape) used in a class with no `extends`.
    pub const E_NO_PARENT_CLASS: Code = Code::new("E0423");
    /// `lateinit` on a scalar-, enum-, or shape-typed property — only a
    /// class/interface (`object`-subtyped) property has no free real default
    /// for `lateinit` to defer past. See `rule:classes/lateinit-restrictions`.
    pub const E_LATEINIT_NOT_OBJECT_TYPE: Code = Code::new("E0424");
    /// `lateinit` on a `?T` property — nullability already spells "may
    /// legitimately hold no value," so there is no second "not yet written"
    /// state left for `lateinit` to add. See `rule:classes/lateinit-restrictions`.
    pub const E_LATEINIT_NULLABLE: Code = Code::new("E0425");
    /// `lateinit` on a promoted constructor parameter — binding the
    /// parameter is already the assignment `rule:classes/definite-property-initialization` requires, so there is
    /// nothing left to defer. See `rule:classes/lateinit-restrictions`.
    pub const E_LATEINIT_PROMOTED_PARAM: Code = Code::new("E0426");
    /// `lateinit` combined with `readonly` on the same property — opposite
    /// promises about when the one allowed assignment happens. See `rule:classes/lateinit-restrictions`.
    pub const E_LATEINIT_READONLY_CONFLICT: Code = Code::new("E0427");
    /// A `lateinit` property read inside a method body with no intervening
    /// write to it and no intervening call on that path since the method's
    /// entry — the one intraprocedural, false-positive-free case `rule:classes/lateinit-read-before-write` proves at compile time; every other case relies entirely on the
    /// § 2 runtime throw.
    pub const E_LATEINIT_READ_BEFORE_WRITE_LOCAL: Code = Code::new("E0428");
    /// An integer literal whose magnitude doesn't fit the width it's being
    /// checked against — too large for `int`/`uint` outright, or exactly the
    /// one magnitude `uint` can never represent regardless of width: a
    /// negative value, since an integer literal's digits are never signed and
    /// the sign comes from a wrapping unary `-`. See `rule:types/arithmetic`.
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
    /// UTF-8 — `string` is guaranteed-valid UTF-8 (`rule:types/bytes`), so a byte
    /// escape's raw output has to actually decode, not just fit in a byte.
    pub const E_STRING_LITERAL_INVALID_UTF8: Code = Code::new("E0431");
    /// A heredoc/nowdoc's closing marker is indented with a mix of spaces and
    /// tabs — PHP 7.3's "flexible heredoc" rule (which this qualifier
    /// mirrors) requires the marker's own indentation to be one or the
    /// other, never both, since a body line's leading whitespace must match
    /// it byte-for-byte to be stripped. See
    /// `nvs_types::string_lit::heredoc_shape`.
    pub const E_HEREDOC_MIXED_INDENT: Code = Code::new("E0432");
    /// A non-blank heredoc/nowdoc body line has less leading whitespace than
    /// its own closing marker — PHP 7.3's "flexible heredoc" rule requires
    /// every body line to start with at least the marker's own indentation
    /// so it can be stripped uniformly. A line that is entirely empty is
    /// exempt from this check. See `nvs_types::string_lit::dedent_heredoc_run`.
    pub const E_HEREDOC_INSUFFICIENT_INDENT: Code = Code::new("E0433");
    /// A `float`, `bool`, `null` or enum array key, at each site that writes
    /// one: an explicit `key =>` in an array literal, an `$a[...]`
    /// subscript, and an `$a[...] = v` target. PHP silently truncates a float,
    /// stringifies `true` to `"1"` and `null` to `""`; `rule:types/arrays` rejects
    /// each outright since each is a silent conversion at the exact
    /// place a mistake becomes a missing row. An enum case is refused one step
    /// further out: `rule:enums/closed-integer-type` makes it a named integer, so the key would be a
    /// backing value two enums can share. An `int`/`uint`/`string` key is
    /// fine — an `int`/`uint` key normalizes to its own decimal string, which
    /// needs no `as` and is not a value conversion.
    pub const E_ARRAY_KEY_INVALID_TYPE: Code = Code::new("E0434");
    /// A `private` interface method (`rule:classes/interface-private-methods`) called from anywhere other
    /// than its own declaring interface's method bodies — it is an internal
    /// helper, never part of the interface's contract, so an implementing
    /// class (or any other interface) cannot see it at all, not even via
    /// `InterfaceName::method()`.
    pub const E_INTERFACE_PRIVATE_METHOD_NOT_VISIBLE: Code = Code::new("E0435");
    /// An `enum` case whose explicit `= expr` value is not an integer literal
    /// (or a negated one). `rule:enums/declaration` makes a case a compile-time integer
    /// constant, not a general constant-expression position.
    pub const E_ENUM_CASE_VALUE_NOT_LITERAL: Code = Code::new("E0436");
    /// An `enum` case whose value — written, or reached by `rule:enums/declaration`'s
    /// auto-increment — does not fit the enum's backing type.
    pub const E_ENUM_CASE_VALUE_OUT_OF_RANGE: Code = Code::new("E0437");
    /// `enum Name: T` where `T` is neither `int` nor `uint` — `rule:enums/one-backing-type`
    /// gives every enum exactly one underlying *integer* type. The `string`
    /// spelling has its own, earlier diagnostic
    /// ([`E_ENUM_STRING_BACKING_UNSUPPORTED`]); this covers the rest.
    pub const E_ENUM_BACKING_NOT_INTEGER: Code = Code::new("E0438");
    /// An argument passed to an `inout $x` parameter that is not a *writable place*
    /// — a bare local or a compile-time-known property. A literal, an
    /// arithmetic result or a call's own result has no storage for the callee
    /// to write back into, so the reference would have nowhere to land. An
    /// array element is refused for a different reason and permanently:
    /// copy-on-write leaves it no address that survives the call
    /// (`nvs_types::expr::args::check_inout_arg` owns why the call-site copy
    /// that would fake one is not offered).
    pub const E_INOUT_ARG_NOT_A_PLACE: Code = Code::new("E0439");
    /// An argument passed to an `inout $x` parameter whose type is not *exactly*
    /// the parameter's. `rule:types/declaration` leaves no room for a conversion here:
    /// the callee writes back through the reference at the declared type, so
    /// anything the caller's storage would have to be converted from on the
    /// way in would have to be converted back on the way out — silently, and
    /// lossily.
    pub const E_INOUT_ARG_TYPE_NOT_EXACT: Code = Code::new("E0440");
    /// A `<...>` type-argument list written after a name that takes no type
    /// parameters. `rule:types/declaration` parks user-declared generics, and two doors
    /// open in that wall — `rule:iteration/concrete-generic-implements`'s compiler-owned generic interfaces,
    /// which `nvs_hir::interfaces::RESERVED` rosters, and a `Core` member
    /// whose spec signature writes one (`Core\Json::decodeAs<T>`), which
    /// `nvs_stdlib::registry::CoreTy::Written` marks. Everything else lands
    /// here: a `type` alias (`rule:statements/nothing-gets-a-second-name` gives one no parameters of its own), a
    /// user-declared method, and a `Core` member that infers its variables
    /// from its arguments instead.
    pub const E_TYPE_ARGS_NOT_GENERIC: Code = Code::new("E0441");
    /// A name or member written with the wrong number of type arguments,
    /// including none at all: `Iterator` on its own is as much a mistake as
    /// `Iterator<int, string>`, since the element type is the whole reason the
    /// parameter exists and `rule:types/declaration` leaves no position untyped. A call site
    /// that omits a member's required list reaches the same rule.
    pub const E_TYPE_ARG_COUNT: Code = Code::new("E0442");
    /// A `foreach` subject that is none of `rule:iteration/foreach-subjects`'s accepted
    /// shapes — an `array<T>`, an `Iterable<T>` or an `Iterator<T>`. A class
    /// reaching neither interface lands here, which is what keeps `foreach`
    /// from being another implicit-dispatch site.
    pub const E_FOREACH_SUBJECT_NOT_ITERABLE: Code = Code::new("E0443");
    /// A `foreach ($x as $k => $v)` key binding over an `Iterable`/`Iterator`
    /// subject. `rule:iteration/two-interfaces` gives a cursor exactly `advance()` and
    /// `current()`; there is no key, and inventing a position counter would
    /// be a second thing `foreach` means.
    pub const E_FOREACH_KEY_ON_CURSOR: Code = Code::new("E0444");
    /// A `yield` in a body that is not a generator's own — at file scope, or
    /// inside an `rule:types/closure-literal` closure. `rule:iteration/generators` confines `yield` lexically to
    /// the generator's own body, which is the stated price of lowering to a
    /// state machine rather than to a coroutine.
    pub const E_YIELD_OUTSIDE_GENERATOR: Code = Code::new("E0445");
    /// A generator — a function whose body contains `yield` — declaring a
    /// return type other than `Iterator<T>`. `rule:iteration/generators`: calling one runs
    /// no user code and returns the state object, which implements exactly
    /// that interface.
    pub const E_GENERATOR_RETURN_TYPE: Code = Code::new("E0446");
    /// A `return expr;` inside a generator. `rule:iteration/one-way-only` makes a generator a
    /// lazy sequence and nothing more — there is no generator return value to
    /// retrieve, so a bare `return;` (stop here) is the only form.
    pub const E_GENERATOR_RETURNS_A_VALUE: Code = Code::new("E0447");
    /// `yield from`, or a `yield` with a `key =>` half. `rule:iteration/one-way-only` rejects
    /// the first as the second spelling of an explicit re-yield loop; § 1
    /// gives `Iterator<T>` no key for the second to produce.
    pub const E_YIELD_FORM_UNSUPPORTED: Code = Code::new("E0448");
    /// A concrete class that reaches an interface method nothing gives a
    /// body — a dispatch to nothing. ADR 0053 § 1's `Iterator<T>` is the
    /// shape that reaches it, its members being bodiless by design.
    pub const E_INTERFACE_METHOD_MISSING: Code = Code::new("E0449");
    /// A block-bodied `fn` closure literal (`rule:types/closure-literal`) with no declared
    /// return type. An expression body *is* its own answer, so it needs no
    /// annotation; a block body would need whole-body return-type inference,
    /// which `rule:types/declaration`'s "nothing is untyped, and no type ever changes by
    /// itself" does not ask the compiler to grow.
    pub const E_CLOSURE_RETURN_TYPE_REQUIRED: Code = Code::new("E0450");
    /// A parameter default (`function f(int $n = ...)`) that is not a literal
    /// of the parameter's own declared type, optionally negated. Novis evaluates
    /// a default once, at signature collection, and materializes it at the
    /// call site that omitted it — so it has to be a constant this compiler
    /// can emit, not PHP's general constant *expression*. See
    /// `nvs_types::defaults`, which owns the accepted set and the shapes
    /// (`null`, an enum case) it is expected to grow next.
    pub const E_PARAM_DEFAULT_NOT_LITERAL: Code = Code::new("E0451");
    /// A parameter with no default declared *after* one that has a default.
    /// Every call supplies arguments positionally, so a required parameter
    /// behind an optional one could never be reached — PHP diagnoses the same
    /// shape.
    pub const E_PARAM_DEFAULT_ORDER: Code = Code::new("E0452");
    /// Something other than an `rule:types/object-top` object literal written at a `Core`
    /// member's trailing options-bag parameter (`rule:core-api/shape-rules` R2). The bag has no
    /// runtime representation — it flattens into one argument per declared
    /// option at the call site — so it must be written out there or omitted
    /// entirely; a variable holding one cannot be passed. `rule:core-api/shape-flattens-at-the-abi`'s
    /// **shape key** parameter is refused here on the same terms and for the
    /// same reason — one flatten, one rule — and the message says "options"
    /// for either, the bag being the all-optional case of the shape.
    pub const E_OPTIONS_NOT_A_LITERAL: Code = Code::new("E0453");
    /// A field name in an options bag that the member does not declare —
    /// usually a typo. Unlike `rule:types/shape-type`'s width subtyping, which accepts an
    /// extra field on purpose, an options bag refuses one: a misspelled option
    /// that is silently ignored is the failure `rule:core-api/shape-rules` R2 exists to prevent.
    /// A **shape key** no arm of an `rule:core-api/shape-parameter` shape parameter declares is
    /// this same code: the merged list is the whole key set either way. So is a
    /// key that belongs to an arm the literal's other values did not select —
    /// § 2's arm selection narrows *which* key set a call is held to, and a key
    /// outside the selected one is still not a key of this call. The two read
    /// differently and are one code on purpose: a second code would ask the
    /// reader to know which arm they were in before they could look it up.
    pub const E_UNKNOWN_OPTION: Code = Code::new("E0454");
    /// `decimal ⊕ float` arithmetic, or `**` with a `decimal` base — `rule:types/arithmetic`. The same rule and the same reason as [`E_INT_UINT_ARITHMETIC`]:
    /// there is no type that represents both operands' values, so one side
    /// must be converted explicitly.
    pub const E_DECIMAL_FLOAT_ARITHMETIC: Code = Code::new("E0455");
    /// A numeric literal placed at `decimal` whose mantissa exceeds 96 bits or
    /// whose scale exceeds 28 — `rule:types/decimal`'s layout. `Core\BigDecimal` (§ 6)
    /// is the type for a value beyond it.
    pub const E_DECIMAL_LITERAL_OUT_OF_RANGE: Code = Code::new("E0456");
    // `E0457` (`E_LITERAL_TYPE_UNCHECKED`) is **retired**, not reused.
    // `rule:types/literal-types`'s
    // atoms intern as real types (`nvs_types::lower::lower_atom`), so there
    // is nothing left for it to refuse.
    /// A `Core` **instance** member written as a static call —
    /// `Core\Regex\Match::text($m)` rather than `$m->text()`. `rule:core-api/shape-rules` R20
    /// gives every `Core` operation exactly one spelling, and this is the one
    /// place two could otherwise reach the same helper: an instance member's
    /// receiver is argument slot 0 at the ABI, so the static spelling would
    /// pass the arity check with the receiver written as an ordinary argument.
    pub const E_CORE_INSTANCE_MEMBER_CALLED_STATICALLY: Code = Code::new("E0458");
    /// A plain `->` on a receiver whose type includes `null` — `?->`, or a
    /// `!= null` test around it, is how a member of one is reached. PHP
    /// throws for this at run time; Novis refuses it while compiling, because
    /// `?T` is one union with no class to resolve a member against. Inside a
    /// block a `!= null`/`== null` test proved the receiver non-`null`
    /// (`nvs_types::locals`' narrowing) this does not fire at all — until
    /// something in that block assigns the local again, which takes the
    /// narrowing back off.
    pub const E_NULLABLE_RECEIVER: Code = Code::new("E0459");
    /// A `#[Json\Derive]` field that is not a same-named constructor parameter.
    /// `rule:core-classes/derive-field-list` makes a decode an ordinary `new`, so every field the codec
    /// reads has to have a parameter to arrive through; `#[Json\Field(skip:
    /// true)]` is the stated way out.
    pub const E_DERIVE_FIELD_NOT_A_PARAMETER: Code = Code::new("E0460");
    /// A `#[Json\Derive]` field whose constructor parameter is declared with a
    /// different type than the property. `rule:core-classes/derive-field-list`: the two lists are one
    /// declaration for a promoted parameter, so a divergence is always written
    /// by hand and always a mistake.
    pub const E_DERIVE_FIELD_TYPE_MISMATCH: Code = Code::new("E0461");
    /// A `secret` property on a class carrying `#[Json\Derive]`. `rule:security/derived-codec-qualifiers`
    /// moves `rule:security/secret-qualifier`'s refusal from wherever the value reached the encoder to
    /// the declaration that put it on the wire contract.
    pub const E_DERIVE_SECRET_FIELD: Code = Code::new("E0462");
    /// A `lateinit` property on a class carrying `#[Json\Derive]`. `rule:core-classes/derive-field-list`: `lateinit` (`rule:classes/lateinit`) is by definition not constructor-assigned,
    /// so it can never be a field.
    pub const E_DERIVE_LATEINIT_FIELD: Code = Code::new("E0463");
    /// A `#[Json\Field(...)]` argument that is not one of `rule:core-classes/derive-field-list`'s
    /// options, or whose value is not a literal of that option's type.
    pub const E_DERIVE_FIELD_ATTRIBUTE: Code = Code::new("E0464");
    /// A type argument written where the member needs a *class* rather than
    /// any type — `Core\Json::decodeAs<int>`. The members that do are
    /// `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS`, and each of them reaches
    /// the written class's runtime descriptor from native code, which only a
    /// class has.
    pub const E_TYPE_ARG_NOT_A_CLASS: Code = Code::new("E0465");
    /// An `==`/`!=` — or a `switch` label, or a `match` arm — whose two static
    /// types are **disjoint**: no single value inhabits both, so the compiler
    /// already knows the answer. `rule:expressions/disjoint-comparison-refused`'s table, and § 6 for the
    /// comparison forms that are not written with the operator.
    pub const E_DISJOINT_EQUALITY: Code = Code::new("E0466");
    /// `+` or `+=` with an array operand. `rule:types/array-combination` removes PHP's array
    /// union operator rather than migrating it — the diagnostic names
    /// `Core\Arr::underlay`, which is what it always meant.
    pub const E_ARRAY_PLUS_UNSUPPORTED: Code = Code::new("E0467");
    /// `Foo::BAR` in *type* position where `Foo::BAR` is declared but is not a
    /// `string`/`int` compile-time constant — `rule:types/constant-in-type-position`. A class constant is
    /// sugar that folds to its own literal type, so it folds only when the
    /// value has a literal type to fold to: a `float` (§ 7 defers those), an
    /// `array`, an object, or an expression that is not a literal at all has
    /// none. A name nothing declares is [`E_UNKNOWN_MEMBER`] instead — that is
    /// a different mistake with a different fix.
    pub const E_LITERAL_TYPE_NOT_CONST: Code = Code::new("E0468");
    /// `"z" as "a"|"b"` — `rule:types/literal-types`: a checked conversion into a closed set
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
    /// subclasses — `rule:core-api/written-visibility`'s levels, enforced. The test is
    /// keyed on the **accessing** class and never on the receiver's static
    /// type: `$other->secret` is legal inside `Secret`'s own body and the
    /// identical line is not at file scope. A name nothing declares anywhere
    /// in the chain is [`E_UNKNOWN_MEMBER`] instead.
    pub const E_MEMBER_NOT_VISIBLE: Code = Code::new("E0471");
    /// `public int $n = "no";` — a property's inline default is evaluated once,
    /// at signature collection, into the constant every fresh instance's slot
    /// is written with (`nvs_types::defaults`), so it has to be a compile-time
    /// constant of the property's own declared type. Its own code rather than
    /// [`E_PARAM_DEFAULT_NOT_LITERAL`] because the two accept different sets:
    /// a property may be defaulted to `[]`, to an enum case or to another
    /// class's `const` — `rule:attributes/payload-is-a-compile-time-constant`'s whole set — and a parameter to a
    /// literal only.
    pub const E_PROPERTY_DEFAULT_NOT_LITERAL: Code = Code::new("E0472");
    /// `$obj as ?SomeClass` — `rule:expressions/nullable-conversion-availability`'s class row: `instanceof` plus
    /// `rule:types/unions-and-mixed`'s narrowing already answers class membership, so the
    /// conversion would be R17's second spelling of a question the language
    /// already has one for. Reported for the written `?T` sugar only, since
    /// § 1 deliberately leaves the `SomeClass|null` union spelling out of the
    /// form.
    ///
    /// **There are no exceptions**, including `Core\Uri` and `Core\Uuid`.
    /// § 3a's `Core\Uri::tryParse` is the member that answers a parse instead
    /// — which the help names for a class in
    /// `nvs_stdlib::registry::TRY_PARSE_CLASSES`.
    pub const E_CLASS_CONVERSION_TARGET: Code = Code::new("E0473");
    /// `++`/`--` on a binding that is not one of `rule:types/arithmetic`'s numeric
    /// types.
    ///
    /// An increment is `± 1` and nothing else. PHP's `$s++` walking a string
    /// through `"a"`→`"b"`→`"aa"` is a divergence this takes deliberately:
    /// § 2 says a declared type never changes and § 4's table has no row that
    /// produces `"b"` from a `string` and a `1`, so there is no arithmetic
    /// here to lower. `nvs_types::expr::operators`' module doc is that
    /// decision's home.
    pub const E_INCREMENT_NOT_NUMERIC: Code = Code::new("E0474");
    /// A `break`/`continue` whose level names no target it can jump to: a
    /// level computed at run time, a `0`, or more enclosing loops than there
    /// are — including the bare `break;` written outside every loop.
    ///
    /// PHP refuses all four at compile time too, and for the same reason:
    /// `break N` resolves to a *statically known* enclosing statement, so a
    /// level that names none has nothing to lower to.
    /// `nvs_types::locals` is where the enclosing depth is counted.
    pub const E_BREAK_LEVEL: Code = Code::new("E0475");
    /// A `match` written with no arms at all.
    ///
    /// PHP parses one and throws `UnhandledMatchError` on every evaluation,
    /// so the construct has no reachable value there either. Novis refuses it
    /// where it is written instead: a `match` is an *expression*, and one
    /// whose every path throws has nothing for the position it sits in to
    /// bind, pass or return. Nothing that worked is lost — a written arm, or
    /// a `throw` expression, says the same thing and says it on purpose.
    pub const E_MATCH_NO_ARMS: Code = Code::new("E0476");
    /// A method called on a receiver whose type names no class: a plain
    /// `object` (`rule:types/grammar`'s opaque top of every class type), an `rule:types/object-top`
    /// shape, a union naming no single class, or a type that can hold no
    /// object at all — a scalar, an `array<T>`, a `void` call's result.
    ///
    /// One code across that whole family, because it is one mistake: a method
    /// is resolved against a class and none of these names one. `rule:types/erased-member-access`
    /// gave the *property* half of an erased receiver a name-keyed runtime
    /// fetch and deliberately stopped there — which is why the property half
    /// splits into a deferral and [`E_RECEIVER_HAS_NO_PROPERTIES`] where this
    /// one does not. A call needs an argument list checked against a
    /// signature and a return type to bind the position it sits in, and no
    /// receiver here supplies either, with no `__call` to fall back on
    /// (`rule:classes/property-observer`). `mixed` is the one receiver deliberately *not* refused:
    /// `rule:types/conversion` makes it the one unchecked position, so it defers.
    ///
    /// The help follows the receiver: narrow one that can hold an object
    /// (`instanceof` proves the class, `as ClassName` converts to it), and
    /// convert or declare `mixed` for one that cannot.
    pub const E_METHOD_ON_ERASED_RECEIVER: Code = Code::new("E0477");
    /// An array element written through an `rule:classes/property-hooks` hooked property:
    /// `$obj->hooked[0] = v`, or any deeper subscript over the same base.
    ///
    /// A hooked property is a pair of accessors, not a slot, so the element
    /// write has nothing to write *into*: `rule:types/arrays` separates the array
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
    /// an `rule:types/object-top` shape, or `rule:types/grammar`'s plain `object`.
    ///
    /// `rule:types/erased-member-access` gave such a property a name-keyed runtime *fetch* and
    /// deliberately stopped there, which is enough for a read and not enough
    /// for a write: `rule:types/arrays` separates the array on the way in, and the
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
    /// it is `rule:php-migration/every-divergence-is-deliberate-and-listed` row 10: PHP appends because the element that is
    /// not there yet reads as `""`, and no rule in Novis makes an absent
    /// element read as a zero value — which is § 7 row 8 one storage kind
    /// along, not a new judgement.
    pub const E_APPEND_IN_READ_POSITION: Code = Code::new("E0481");
    /// `$x[…]` where `$x` is not an `array<T>`, and `[…] = $x;` — `rule:types/grammar`.3's destructuring — for the same reason: every leaf is an element
    /// read, so a value with no elements has nothing to take apart.
    ///
    /// `rule:types/arrays` checks an element read and write against the array's
    /// *declared* element type, so a base that declares none — a `mixed`, a
    /// scalar, an object, or a `?array<T>` a `!== null` test has not
    /// narrowed — has no element to name and nothing to check against.
    /// PHP answers `null` with a warning for most of these, which is § 7
    /// row 8's family; Novis refuses at check time instead. A `string` is not
    /// an exception: `rule:types/string-is-utf8` indexes one by grapheme cluster through
    /// `Core\Str`, not through a subscript.
    pub const E_SUBSCRIPT_ON_NON_ARRAY: Code = Code::new("E0482");
    /// `&value` as an element of an array literal, or `[int &$x] = $pair;`
    /// as a destructuring leaf — the same element from the other side, and
    /// the same answer.
    ///
    /// PHP's `[&$x]` stores a reference, so writing the element writes
    /// `$x` too. Novis has nowhere to put one: `rule:types/implicit-capture` removed
    /// by-reference capture, so no binding aliases another, and `rule:classes/two-copy-depths`
    /// fixes what a copy means, so an element is a copy at the point the
    /// literal is evaluated. An aliasing element would therefore have no
    /// owner in either rule — it is not a lowering that is missing, it is
    /// a thing the language does not have. Write the value; to share one
    /// mutable cell, put it in an object, exactly as `rule:types/implicit-capture`'s own
    /// worked example does.
    pub const E_ARRAY_ELEMENT_BY_REFERENCE: Code = Code::new("E0483");
    /// `[...$x]` where `$x` is not an `array<T>`.
    ///
    /// A spread element contributes the subject's *entries* to the literal
    /// being built, so a subject with no entries has nothing to contribute.
    /// PHP's `[...$s]` over a string is a `TypeError` at run time; Novis's
    /// element types are declared, so it is a diagnostic instead. Where the
    /// literal does have an expected element type the mismatch is reported
    /// as an ordinary [`E_TYPE_MISMATCH`] against `array<T>` instead, which
    /// names both array types and is the better message — so this code is
    /// only what a position naming no `array<T>` at all is left with, a
    /// `mixed` binding or parameter being the reachable one.
    pub const E_SPREAD_SUBJECT_NOT_AN_ARRAY: Code = Code::new("E0484");
    // `E0485` is retired and is never reused: it refused a `name:` argument at
    // a target whose signature carried no parameter names, and `rule:core-api/shape-rules` R2
    // leaves no such signature — a `Core` row's names are
    // `nvs_stdlib::registry::CoreMethod::names`, the synthesized `Throwable`
    // constructor's are `message` and `options`, and a reserved interface's
    // are the ones its own ADR writes. A name reaching no parameter is
    // [`E_UNKNOWN_ARG_NAME`] everywhere.
    /// `name: value` naming no parameter a call can fill by name — either the
    /// callee declares no parameter of that name at all, or the name reaches
    /// its `...$rest` tail.
    ///
    /// PHP collects an unmatched named argument into a variadic parameter as a
    /// string-keyed entry. Novis's variadic tail is an ordinary `array<T>` built
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
    /// `foreach (… as inout $v)` over a subject that is not a plain variable
    /// holding an `array<T>` — a call's result, a literal, a property, or an
    /// `Iterable`/`Iterator`.
    ///
    /// `inout $v` writes each element back where it came from, so there has to be
    /// a slot to write to. PHP refuses the same shapes, and a cursor is the
    /// one it names outright ("an iterator cannot be used with foreach by
    /// reference").
    pub const E_FOREACH_INOUT_SUBJECT: Code = Code::new("E0490");
    /// A `foreach (… as inout T $v)` whose `T` is not the subject's element type
    /// exactly.
    ///
    /// A by-value binding may widen — reading an `array<Dog>` as an `Animal`
    /// is sound — but a by-reference one also *writes*, and writing an
    /// `Animal` into an `array<Dog>` is not. The two directions meet only at
    /// the element type itself.
    pub const E_FOREACH_INOUT_ELEMENT_TY: Code = Code::new("E0491");
    /// A method whose body contains `yield` declaring an `inout $x` parameter.
    ///
    /// A by-reference parameter addresses a cell the *call site* stages for
    /// the duration of the call. Calling a generator runs none of its body —
    /// it allocates the state object and returns (`rule:iteration/generators`) — so that cell
    /// is gone before the first `advance()`, and there is nothing sound for
    /// the suspended frame to keep addressing.
    pub const E_GENERATOR_INOUT_PARAM: Code = Code::new("E0492");
    /// A `fn` closure literal declaring an `inout $x` parameter.
    ///
    /// A by-reference parameter is a contract between a call site and a
    /// declaration, and a closure's type is `callable` — `rule:types/callable-absorbs-closure` keeps it
    /// opaque, carrying no parameter list at all, so no call site can know to
    /// stage a cell. The closure may also outlive every frame in scope where
    /// it was written.
    pub const E_CLOSURE_INOUT_PARAM: Code = Code::new("E0493");
    /// An `rule:types/object-literal` object literal writing one field name twice —
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
    /// PHP warns and yields `null` here; `rule:php-migration/every-divergence-is-deliberate-and-listed` row 13 makes it a
    /// check-time error instead, for row 8's reason: a declared type is what
    /// makes the answer knowable before the program runs, and nothing in Novis
    /// makes an absent thing read as a zero value. `mixed` is the one receiver
    /// that keeps PHP's *timing* — `rule:types/conversion`'s one unchecked position, so
    /// it defers to `rule:types/erased-member-access`'s name-keyed fetch and its catchable throw.
    pub const E_RECEIVER_HAS_NO_PROPERTIES: Code = Code::new("E0495");
    /// A class named through a value rather than written, and the two
    /// `instanceof` right-hand sides that name no class at all: the dynamic
    /// form `$x instanceof $name`, `new $name()` and `$name::f()`, plus a
    /// `Core` namespace class or an enum on the right of `instanceof`.
    ///
    /// The dynamic form is `rule:types/conversion`'s rule: a class name is written, never
    /// computed, which is the same line `$$var` and `eval` are already on. Its
    /// three spellings share one report
    /// (`nvs_types::expr::members::reject_dynamic_class_name`) because they are
    /// one mistake — `rule:php-migration/every-divergence-is-deliberate-and-listed` row 14 says so of the `instanceof` one, and a
    /// second code for the same rule at `new` would be a distinction the
    /// language does not make. A `Core` **namespace** class — one declaring
    /// neither a slot nor an instance member — is a name for calling static
    /// members through, so no value is ever an instance of it; a `Core` class
    /// that does have instances is tested against the descriptor
    /// `nvs_stdlib::class_descriptors` publishes and is not here at all. An
    /// enum is a value type (`rule:enums/closed-integer-type`) and no value of one is ever an object,
    /// so the test has nothing to walk. A written name that resolves to
    /// *nothing* is not here: that is the ordinary `E0303`, exactly as
    /// `new Undeclared()` already reports it.
    pub const E_INSTANCEOF_NOT_A_CLASS: Code = Code::new("E0496");
    /// `instanceof` over a left-hand side whose declared type can hold no
    /// object at all — `int $n = 1; $n instanceof Box;`.
    ///
    /// PHP answers `false`, having no declaration to read; `rule:php-migration/every-divergence-is-deliberate-and-listed` row 14
    /// refuses it instead, for the reason `rule:expressions/one-equality-operator` refuses two statically
    /// disjoint types under `==` — the declaration already answered, so the
    /// test is dead code that reads as a live question. `mixed`, `object`, a
    /// shape, a class and any union holding one all keep the run-time test.
    pub const E_INSTANCEOF_SUBJECT_NOT_OBJECT: Code = Code::new("E0497");
    /// An `isset(...)` operand that names no storage — `isset(f())`,
    /// `isset($a + 1)`, `isset(Foo::BAR)`.
    ///
    /// PHP refuses the identical shape at compile time, and its own message
    /// names the replacement: *"Cannot use isset() on the result of an
    /// expression (you can use `null !== expression` instead)"*. `rule:classes/unset-is-refused-on-a-property`
    /// fixes `isset($x)` as `$x != null`, so an operand that is already a
    /// value rather than a place has nothing `isset` can ask that `!= null`
    /// does not ask more plainly.
    ///
    /// The accepted set is PHP's: a variable, a subscript, a property (`?->`
    /// included), a static property, and any of those in parentheses.
    pub const E_ISSET_NOT_A_VARIABLE: Code = Code::new("E0498");
    /// `static::$prop` — late static binding on a *static property*, whose
    /// storage Novis resolves at compile time.
    ///
    /// PHP re-resolves the name against the **called** class, so a subclass
    /// that redeclares the property gets its own storage through this
    /// spelling and the parent's through `self::`. Novis's slot is fixed where
    /// the access is written (`nvs_ir::ir::StaticProp`), which answers the
    /// same as PHP for every class that does *not* redeclare and differs
    /// silently for one that does — so the spelling is refused rather than
    /// left to diverge, per `AGENTS.md`'s priority 2. `self::$prop` and a
    /// written class name both say exactly which storage is meant and are
    /// unaffected.
    ///
    /// **The last code in the E04xx band**, which is full. The band decision
    /// is [`E_ELEMENT_WRITE_ROOT_NOT_A_PLACE`]'s: the types band continues at
    /// `E07xx`, and `E0500` is never issued, its own digits reading as the
    /// IR-and-codegen band. `docs/adr/README.md` § *Decisions taken at project
    /// start* is the one home for why.
    pub const E_STATIC_PROPERTY_LATE_BOUND: Code = Code::new("E0499");

    // --- E05xx IR and codegen ----------------------------------------------
    /// The IR verifier rejected a function. Always an Novis bug.
    pub const E_IR_INVALID: Code = Code::new("E0501");
    /// Cranelift could not compile a function.
    pub const E_CODEGEN_FAILED: Code = Code::new("E0502");

    // --- E06xx configuration and capabilities ------------------------------
    /// An `nvs.toml` directive that does not exist, or an invalid value.
    pub const E_BAD_DIRECTIVE: Code = Code::new("E0601");
    /// A `Core\Config::set` the directive's changeability class refuses: a `System`
    /// directive, widening a `RuntimeTighten` one, or exceeding a hard ceiling.
    pub const E_CAPABILITY_DENIED: Code = Code::new("E0602");
    /// A per-request limit was exceeded.
    pub const E_LIMIT_EXCEEDED: Code = Code::new("E0603");
    /// The same directive set twice in one configuration file. `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one` refuses
    /// it so that no assignment in a root-owned file is ever silently shadowed;
    /// across an `[[include]]` the same key is an override instead (`rule:config/later-wins-and-every-override-is-recorded`).
    pub const E_DUPLICATE_DIRECTIVE: Code = Code::new("E0604");
    /// A file the configuration tree names cannot be read: a `--config` that does
    /// not exist, a non-`optional` `[[include]]` that does not, or an I/O failure
    /// on one that does. `rule:config/ownership-is-the-trust-boundary` makes every one of these a hard refusal —
    /// `optional` covers absence and nothing else, because otherwise a stray
    /// `chmod` silently drops half a configuration and the server comes up
    /// looking healthy.
    pub const E_UNREADABLE_CONFIG: Code = Code::new("E0605");
    /// An `[[include]]` reached a file already on the chain that pulled it in, or
    /// nested deeper than `rule:config/include-takes-a-path-or-a-dir`'s cap of eight. The refusal names the whole
    /// chain, because the cycle is a property of the path and not of its last file.
    pub const E_INCLUDE_CYCLE: Code = Code::new("E0606");
    /// A file the configuration tree names fails `rule:config/ownership-is-the-trust-boundary`'s trust boundary:
    /// it is owned by an account that is neither this one nor an administrative
    /// one, or an account outside those can write it — the file itself, or the
    /// directory holding it, or the directory an absent `optional` include would
    /// appear in. Whoever can write one file in the tree can grant themselves
    /// every capability it carries, so this is a refusal to start rather than a
    /// warning, and it is re-run on every `nvs ctl reload`.
    pub const E_UNTRUSTED_CONFIG: Code = Code::new("E0607");
    /// A `_file` sibling of a secret directive — `[db.<name>] password_file` —
    /// names a file the configuration cannot take a value from: it is
    /// set alongside the directive it stands in for, or its content is empty,
    /// whitespace-only, larger than `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s 64 KiB cap, or not valid
    /// UTF-8. Every one of those is a refusal to start rather than a value
    /// carried forward: an empty credential otherwise fails at the first
    /// request instead of at boot, and the size cap catches a path pointed at
    /// the wrong thing. A secret file that fails the § 6 trust boundary is
    /// `E0607` like any other configuration input, and one that cannot be read
    /// at all is `E0605`.
    pub const E_BAD_SECRET_FILE: Code = Code::new("E0608");
    /// An `[[app]]` block that cannot be keyed on an entry file path: it names
    /// both `root` and `entry`, or neither, or it resolves to a path another
    /// block already claimed. `rule:config/an-application-is-its-entry-file-path` makes an application *be* its entry
    /// file path, so a block with no usable key silently covers nothing and
    /// hands every application it was meant for the global configuration
    /// instead — including one it was written to narrow. Two blocks on one path
    /// are § 2's duplicate rather than a refinement: specificity is what orders
    /// the layering, and equal paths have no order to decide with. A key naming
    /// something that cannot be examined at all is `E0605`, like any other
    /// configuration path.
    pub const E_BAD_APP_BLOCK: Code = Code::new("E0609");
    /// An `[[app]]` block asking for more than the host allows: a value in
    /// `[app.limits]` above the global `[limits.hard]`, or an `[app.limits.hard]`
    /// raising its own ceiling above it. `rule:config/an-app-block-may-widen-bounded-by-the-global-ceiling` lets a block widen
    /// `[app.limits]` and lower its own ceiling, but leaves the bound where ADR
    /// 0005 put it — `[limits.hard]` is the host's answer and an application
    /// cannot exceed it. The refusal is at boot and the value is never clamped,
    /// exactly as a `Core\Config::set` above the ceiling is refused rather than
    /// reduced: a clamp would leave a block reading as though it got what it
    /// asked for. A value that is not a quantity at all is `E0601`, from the one
    /// parser both this check and `Core\Config::set` share (`rule:config/ini-set-is-core-config-set`).
    pub const E_APP_ABOVE_CEILING: Code = Code::new("E0610");

    /// A `[[schedule]]` entry the scheduler could not arm: no `name` or a
    /// duplicate one, no `scope`, a `cron` outside `rule:config/cron-is-five-fields-and-nothing-more`'s five-field
    /// dialect, a `script` outside the `script.spawn` roots, or `scope =
    /// "fleet"` with no shared store to hold § 3's lease. Every one of them is
    /// refused at boot rather than at the first fire, because a schedule's
    /// failure mode is silence: an entry that never fires looks exactly like
    /// one whose interval has not come round yet, and the operator finds out
    /// from the work that did not happen. A path outside the roots is this
    /// code and not `E0602` — nothing has been denied at a door yet, the
    /// configuration simply does not say the file may be run.
    pub const E_BAD_SCHEDULE: Code = Code::new("E0611");

    /// An `[http]` pair with no correct meaning: `rule:http-server/cors-is-closed-until-origins-are-named`'s `origins =
    /// ["*"]` with `credentials = true`, or § 3's `same_site = "None"` with
    /// `secure = false`. Both are refused rather than warned about because
    /// neither is a weak policy — every browser rejects both outright, so the
    /// deployment holds an access-control rule that does not run and nothing
    /// tells it so. This is the boot half; inside a request the same two
    /// combinations make `Core\Config::set` return `false` and leave the value
    /// unchanged (`rule:config/three-changeability-classes`'s existing rule), and the condition behind both
    /// halves is written once in `nvs_config::http`. Each half of a pair is
    /// legitimate alone — a wildcard origin is an ordinary public API — so
    /// there is no refusal here for a single key.
    pub const E_MEANINGLESS_HTTP_PAIR: Code = Code::new("E0612");

    /// A `[log] target` that is none of `rule:errors/engine-floor`'s destinations —
    /// `stderr`, `file:<path>` or `syslog` — including a `file:` with no path
    /// behind it. Refused at the boot that reads the tree rather than at the
    /// first record written through it, because the one moment the engine
    /// cannot report a configuration mistake is the moment it is already
    /// reporting a failure: tier 4 is the floor, and a diagnostic raised there
    /// would displace the record it exists to write. The grammar behind this
    /// refusal is `nvs_config::log::Target`, which is also what
    /// `nvs_runtime::Ctx::write_log_record` resolves a target through, so a
    /// spelling accepted here is a destination that opens.
    pub const E_UNSPELLED_LOG_TARGET: Code = Code::new("E0613");

    /// A `[log] level` that is none of `rule:errors/log-level`'s levels — refused at the
    /// same boot and for the same reason as `E0613` beside it, but against the
    /// opposite failure: an unspelled destination would route records nowhere,
    /// while an unspelled level leaves the floor at `Debug` and writes
    /// everything. An operator who wrote `level = "warning"` believes they
    /// raised it and did not, and no record they collect afterwards says so.
    /// The grammar is `nvs_render::Level::of`, which is also what
    /// `nvs_runtime::Ctx::write_log_record` resolves the floor through.
    pub const E_UNSPELLED_LOG_LEVEL: Code = Code::new("E0614");

    /// A `[log] format` that is neither of `rule:errors/renderings`'s two — the third
    /// refusal of the same block and for the third reason. An unspelled
    /// destination routes records nowhere and an unspelled level widens what is
    /// collected; an unspelled *rendering* leaves them as JSON Lines, so the
    /// deployment collects exactly the right records in the shape it asked not
    /// to have, and the pipeline reading them is the thing that breaks.
    ///
    /// § 3's own paragraph is why the near miss is worth a diagnostic rather
    /// than a fallback: the directive has **two** values and not three, because
    /// HTML is a rendering the response sink selects and never a log target's.
    /// An operator who writes `format = "html"` has asked for something that
    /// exists, somewhere this is not.
    pub const E_UNSPELLED_LOG_FORMAT: Code = Code::new("E0615");

    /// A written `Core\Cap::has("…")` naming something that is not a capability
    /// — `rule:security/capability-roster-is-closed`'s roster is closed, and § 6's whole point is that the
    /// answer decides which branch a package takes. A misspelling folds to
    /// `false` and so reads as *not granted*, which is the same answer the
    /// correct spelling gives on a deployment that granted nothing: the branch
    /// simply never runs, on every machine, and nothing at run time can tell
    /// the two apart. That is `E0798`'s reasoning one class over, and it is why
    /// only a *written* name is refused — a computed one keeps `rule:expressions/intrinsic-literals`'s
    /// rule that nothing is refused for being dynamic, and answers `false` at
    /// run time. In this band rather than the types one because what it checks
    /// is a capability name, which is `nvs_config::capability::Cap`'s roster and
    /// the same table a `[grants]` line is read against.
    pub const E_NOT_A_CAPABILITY: Code = Code::new("E0616");

    /// A `[queue]` block the runtime could not arm: no `connection`, one naming
    /// a `[db.<name>]` block the tree does not hold, a `max_attempts` of `0`,
    /// or a `visibility` of `0`. Refused at boot for `E0611`'s reason one
    /// subsystem over — a queue fails silently by construction, since a job
    /// that is never claimed looks exactly like one whose turn has not come,
    /// and the operator learns about it from the work that did not happen. A
    /// `connection` naming nothing is this code and not `E0601`: the value is a
    /// well-formed name, and what is wrong with it is a fact about the rest of
    /// the tree rather than about the value.
    pub const E_BAD_QUEUE: Code = Code::new("E0617");

    /// A **written** `Core\Db::open` host that the compiling machine's
    /// `db.open` grant does not cover — `rule:core-classes/db-literal-query-checking`'s second sentence, and
    /// the only capability question asked before a program runs.
    ///
    /// It refuses nothing `nvs_runtime::capability::require` would have
    /// allowed: the same grant list, walked by the same
    /// `nvs_config::capability::Capabilities::allows_host`, so this is `rule:expressions/preparation-preserves-behaviour`'s earlier answer and never a different one. It is therefore asked
    /// only where both halves are facts at check time — a literal host, and a
    /// configuration this machine actually read. A computed host, or a check
    /// run with no configuration in front of it, says nothing and leaves the
    /// refusal to the door. In this band rather than the types one for
    /// `E0616`'s reason: what it reads is a grant, not a type.
    pub const E_UNGRANTED_HOST: Code = Code::new("E0618");

    /// A `[server]` wait that would never end: `false`, or `0`. `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s
    /// headline is that Novis never waits forever and `rule:http-server/the-server-block-is-boot-class` states its
    /// four inbound waits as finite with nothing configured, so `false` — which
    /// removes a ceiling everywhere else in the tree (`rule:config/three-changeability-classes`) — has no
    /// meaning here and is refused rather than read as a default. `0` is the
    /// same refusal from the other side: a wait that expires as it is armed
    /// closes every connection before it can say anything, which is a server
    /// that accepts and answers nothing.
    ///
    /// A value that is not a duration at all is `E0601` in
    /// `nvs_config::value`'s own words; this code is only for a well-formed
    /// duration whose *magnitude* is the problem.
    ///
    pub const E_UNBOUNDED_WAIT: Code = Code::new("E0619");

    /// A `[server] listen` entry the server cannot bind, or a written array
    /// with nothing in it. `rule:http-server/the-server-block-is-boot-class`'s array is one flat list whose entries
    /// are a `host:port` or an absolute path meaning a Unix socket, and the
    /// overload is unambiguous because no address can begin with a separator.
    ///
    /// A host *name* is refused rather than resolved: a name that answers with
    /// two addresses is two sockets rather than one, and resolving it at boot
    /// makes the server's start depend on a nameserver being up. An empty array
    /// is refused for the same reason `E0619` refuses `false` — it is a
    /// deployment that accepts nothing, and leaving the key out is how § 5's
    /// own default is kept.
    pub const E_BAD_LISTEN: Code = Code::new("E0620");

    /// A `[[server.mount]]` block that does not resolve to a mount. `rule:http-server/a-path-is-never-derived-from-a-url`
    /// makes the set of paths the server can execute something enumerated
    /// before it accepts anything, so every way a block can fail to name one is
    /// a boot refusal rather than a mount quietly missing from the table.
    ///
    /// It covers the shape — naming both `scan` and `entry` or neither,
    /// matching on neither `prefix` nor `host`, a `{2}` whose glob has one `*`
    /// — and what only the disk answers: an entry that is not there, one that
    /// resolves outside `[server] root`, a captured segment `rule:errors/path-component-refusals`
    /// refuses, and two mounts answering at one key. The first set is checked
    /// wherever the tree is, so `nvs config check` reports it on a machine that
    /// holds none of the files; the second needs the tree it mounts.
    pub const E_BAD_MOUNT: Code = Code::new("E0621");

    /// A `[server] max_in_flight` written as `0`. `rule:http-server/the-server-block-is-boot-class`'s ceiling is a
    /// safety valve, and that spelling asks for a process that accepts a
    /// connection and then refuses every request on it — the same deployment
    /// `E0620` refuses when `listen` is written empty, reached from the other
    /// end. `false` is not the other half of this refusal because the key is a
    /// plain count rather than a `Setting`: `rule:config/three-changeability-classes`'s ceiling-removing
    /// spelling does not typecheck against it, so removing this valve is not
    /// something the file can say.
    ///
    /// Only the written magnitude is this code's: the *effective* ceiling is an
    /// arithmetic against the memory budget
    /// (`rule:http-server/admission-is-arithmetic-not-a-number`
    /// ) and a configured number the budget cannot afford is clamped and
    /// logged rather than refused, because a server that will not boot because
    /// two directives disagree is the worse outage.
    pub const E_NO_ADMISSION: Code = Code::new("E0622");

    /// A `[server] health_path` that is not an absolute path: `healthz`,
    /// `/healthz?verbose=1`, or `/` on its own. `rule:http-server/the-server-block-is-boot-class`'s probe answers
    /// one exact URL ahead of the mount table, so what is written here is
    /// reserved from every application this server mounts — which is why the
    /// key is off by default, and why a spelling no request could ever carry
    /// is refused rather than reserved and then never reached. A relative path
    /// cannot equal a request target, a query or a fragment is not part of the
    /// path one is matched on, and `/` reserves every mount's own entry.
    ///
    /// Not `E0621`'s refusal reached from another direction: a mount path is a
    /// prefix that has to resolve inside `[server] root` on disk, and this is a
    /// whole request target that reaches no filesystem at all.
    pub const E_BAD_HEALTH_PATH: Code = Code::new("E0623");

    /// An `[http.cookies] same_site` that is none of `rule:http-server/cookies-are-secure-httponly-and-lax`'s three
    /// spellings — `Strictly`, `lax=true`, or a value left over from another
    /// framework's own key.
    ///
    /// Refused rather than defaulted because this block's whole job is to
    /// state what a cookie written with no options is: a browser drops an
    /// attribute it cannot parse and falls back to *its* default, so a tree
    /// with a fourth spelling has written a policy that the deployment
    /// believes is in force and nothing else does. It is also what lets
    /// `nvs_config::http::Cookies` resolve the key without an
    /// "or something else" arm, and so without ever repairing one
    /// ([0095](/docs/decisions/0095.md)).
    ///
    /// Not `E0612`'s refusal reached from another direction: that one is a
    /// *pair* of individually meaningful values, and this is one value that
    /// has no meaning by itself — which is why it is asked first, the pair
    /// question being undecidable over a `same_site` nobody can read.
    pub const E_BAD_SAME_SITE: Code = Code::new("E0624");

    /// An `[http.headers]` policy value a header line cannot carry: a `\r\n`
    /// in a `referrer_policy`, a `content_security_policy` or a
    /// `permissions_policy`, or any other byte outside printable ASCII.
    ///
    /// These three are written onto every response verbatim, so a control
    /// character in one is a response-splitting attempt against every request
    /// the server will answer. `nvs_server::secure` already refuses to spell
    /// one and falls back to the shipped default, which is the right answer
    /// for a running server and the wrong one for a boot: the deployment
    /// believes the policy it wrote is in force, the shipped default is what
    /// is actually emitted, and nothing anywhere says so. Refusing at boot is
    /// what makes that fallback unreachable from a server that started, which
    /// is why both exist.
    ///
    /// Not `E0612`'s or `E0624`'s refusal reached from another direction:
    /// both of those are about a value's *meaning* under `rule:http-server/cors-is-closed-until-origins-are-named` and `rule:http-server/cookies-are-secure-httponly-and-lax`, and
    /// this is about whether the bytes can be transmitted at all.
    pub const E_UNCARRIABLE_HEADER: Code = Code::new("E0625");

    /// `[session] backend` names a store a session may not live in.
    ///
    /// `rule:concurrency/the-local-tier-cannot-hold-what-must-be-coherent` removed both weak cache tiers from the candidates and said
    /// the removal is "enforced rather than documented"; this code is that
    /// enforcement, and `rule:http-server/session-backend-is-shared-or-db-and-local-is-refused-at-boot` is where the roster it checks against is
    /// written. A session read on one core and written on another must see one
    /// value, and neither weak tier can give one — so a deployment that wrote
    /// `local` has an authentication surface that forgets people at a rate set
    /// by which core accepted the request, and one that wrote `process` has the
    /// same surface at the distance that tier reaches: it holds until the
    /// deployment runs a second process, or until this one restarts.
    ///
    /// Refused where the key is written rather than where a session is started,
    /// for `E0613`'s reason applied to a worse failure: a session that vanishes
    /// looks like a user signing themselves out, so nothing in the running
    /// system ever reports it.
    pub const E_SESSION_BACKEND: Code = Code::new("E0626");

    /// `[metrics] exporter` or `[trace] exporter` names no exporter that block
    /// has.
    ///
    /// `rule:observability/metrics-and-trace-blocks-are-system` gives metrics a scrape (`prometheus`) or a push (`otlp`)
    /// and gives a trace only the push, with `false` the disabled state for
    /// both. The asymmetry is the section's, not a limitation of the exporters:
    /// a scrape answers with a series' current value, and a span is a finished
    /// record with no current value to answer with.
    ///
    /// Refused where the key is written rather than where an export would run,
    /// for `E0613`'s reason. Both blocks are `System`, so the value in force is
    /// the one the boot read, and a wrong one produces silence — a collector
    /// nothing writes to is indistinguishable from a deployment with nothing to
    /// say, so every dashboard over it is empty rather than wrong and nothing
    /// reports why.
    pub const E_UNSPELLED_EXPORTER: Code = Code::new("E0627");

    /// `[trace] sample` is not a fraction of one.
    ///
    /// `rule:observability/metrics-and-trace-blocks-are-system` writes the head sample as `0.0` to `1.0` inclusive. A
    /// number outside that names no smaller or larger sample — it names
    /// nothing, and what a reader does with `sample = 5` depends on which side
    /// of a comparison it lands on, where "record everything" and "record
    /// nothing" are both plausible readings of the same value.
    ///
    /// Non-finite is refused here rather than left to that comparison, because
    /// TOML spells `nan` and `inf` and every ordering against a `NaN` is false:
    /// a sample written as one would read as "record nothing" through the same
    /// expression that reads `0.0` that way.
    pub const E_SAMPLE_NOT_A_FRACTION: Code = Code::new("E0628");

    /// `[control] socket` names something that would be reachable over a
    /// network rather than a local endpoint.
    ///
    /// `rule:config/no-network-control-surface` has no TCP listener in it, in either direction of
    /// configuration, and § 3's reasoning is why: the socket's owner and mode
    /// *are* the authentication, so a control surface that arrives over a
    /// network has no authentication at all. `socket = "127.0.0.1:9000"` is
    /// the shape that says so out loud, and it is refused at boot rather than
    /// read as the relative path a host and a port happen to spell — a file
    /// called `127.0.0.1:9000` is a legal name on Unix, and creating one is
    /// the reading that leaves an operator believing they bound a port.
    ///
    /// Every other value that names no local endpoint — a list, a float, a
    /// bare `true` — is refused under this same code rather than a second one,
    /// because the directive has exactly two legal shapes and one reason to
    /// have them: a value that is not one local endpoint, and is not `false`,
    /// leaves the operator believing they configured a control surface.
    ///
    /// Not `E0627`'s refusal reached from another direction: that one is an
    /// exporter *sink* the tree does not know, and this one is a value the
    /// tree understands perfectly and is not allowed to accept.
    pub const E_NETWORK_CONTROL_SOCKET: Code = Code::new("E0629");

    /// `nvs service` was asked to store an argv that names something other
    /// than a server.
    ///
    /// `rule:packaging/the-installer-is-a-sink`'s first two rows, under one code because they are one
    /// reason: the trailing argv is executed by a privileged account at every
    /// boot until somebody removes it, so what it names has to be a program
    /// that *stays running* and carries no testing hook. A subcommand outside
    /// the closed `serve`/`run` allowlist exits immediately, which every
    /// service manager reports as a crash loop forever; `--fault-inject` is
    /// `nvs-cli`'s own contained-panic hook, which that flag's doc comment
    /// says must never be reachable from a served request, and a service
    /// carrying it is exactly that with a privileged account attached.
    pub const E_SERVICE_ARGV_NOT_ALLOWED: Code = Code::new("E0630");

    /// A path in the argv `nvs service` was asked to store, or in one of its
    /// own options, is relative — or the argv names no `--config` at all.
    ///
    /// `rule:packaging/the-installer-is-a-sink`'s third and last-but-two rows, under one code because that
    /// section says outright that they are the same failure: a service starts
    /// in `System32` under a minimal environment, so a relative `--config` is
    /// a guaranteed first-boot failure surfacing as an opaque service-manager
    /// error, and an argv with no `--config` at all falls back to
    /// `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`
    /// 's `./nvs.toml` — the same failure one step less visible, because it
    /// makes the service's configuration a property of whatever directory the
    /// manager happened to start it in.
    ///
    /// Not `E0631`'s neighbour `E0632` reached from another direction: this
    /// one is about a path the service could not *find*, and that one about
    /// output it would have nowhere to put.
    pub const E_SERVICE_PATH_NOT_ABSOLUTE: Code = Code::new("E0631");

    /// `nvs service install` was given neither a `--log-file` nor a config
    /// naming a `[log]` destination.
    ///
    /// `rule:packaging/the-installer-is-a-sink`'s fourth row and § 4's *Output*: a service has no console
    /// handle, so the process's stderr is discarded, and a refused compile or
    /// a `FATAL` under this argv would leave no trace anywhere at all. The
    /// installer refuses rather than picking a destination, because a log file
    /// nobody was told about is the second place an administrator looks and
    /// the first place they do not.
    pub const E_SERVICE_OUTPUT_GOES_NOWHERE: Code = Code::new("E0632");

    /// An `--account` password was passed to `nvs service` on the command
    /// line.
    ///
    /// `rule:packaging/the-installer-is-a-sink`'s fifth row: a command line is readable by other users on
    /// the box, so the value is prompted for instead and is `secret` in
    /// `rule:security/secret-qualifier`'s
    /// sense for its whole life. The option exists in order to be refused by
    /// name — a bare "unrecognized argument" would read as a spelling mistake
    /// and send the operator looking for the right flag.
    pub const E_SERVICE_PASSWORD_ON_A_COMMAND_LINE: Code = Code::new("E0633");

    /// `nvs service` was run from an
    /// `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`
    /// bundle.
    ///
    /// `rule:packaging/a-bundle-may-not-install-itself`. A bundle is a single trust domain because the person who
    /// downloads and runs it is the only principal involved; installing a
    /// service creates a **second** principal — a privileged account executing
    /// that payload at every boot, with no operator having read what it
    /// contains — which is the boundary 0048 § 1 declined to cross, arrived at
    /// from the other side.
    pub const E_SERVICE_FROM_A_BUNDLE: Code = Code::new("E0634");

    /// `[cache.shared] url` or `[db.<name>] host` names a Unix-domain socket on a
    /// build that carries no `AF_UNIX` transport.
    ///
    /// `rule:config/a-unix-spelling-with-no-af-unix-transport-refuses-at-boot`.
    /// The transport is `#[cfg(unix)]`: `AF_UNIX` does exist on Windows, but the
    /// reactor's I/O layer does not carry it, so the spelling names a store this
    /// binary has no way to open — and every request until someone notices is one
    /// this refusal would have prevented.
    ///
    /// Not `E0626`'s refusal reached from another direction — that one is the
    /// same question answered the other way round. A session `backend = "db"`
    /// names a store the roster admits whose second half is unwritten, so
    /// refusing it at the key would claim the decision was wrong rather than that
    /// the build has not caught up; a Unix socket on a platform with no `AF_UNIX`
    /// is not waiting for this project, and the deployment has to be spelled
    /// differently. Reading it as loopback TCP instead is refused because a
    /// configuration that reads as one transport and runs as another is
    /// invisible in exactly the review that would have caught it.
    pub const E_NO_UNIX_TRANSPORT: Code = Code::new("E0635");

    /// A `[server] workers` written as `0`.
    ///
    /// `rule:http-server/the-accept-fan-out-is-one-worker-per-core`
    /// : every listening socket this server binds is accepted on by a worker,
    /// so a count of zero is a process that takes the addresses the tree names
    /// and then answers nobody on any of them — `E0620`'s empty `listen`
    /// reached from the other end.
    ///
    /// Only zero is refused. The key is the bound rather than a request for
    /// one, so a count above what the machine answers
    /// `available_parallelism` with is what the operator asked for and is
    /// started: a heuristic that knew better than the block would make the
    /// core count something the file cannot state.
    pub const E_NO_WORKERS: Code = Code::new("E0636");

    /// A **written** `Core\Queue::purge` queue name that the compiling
    /// machine's `queue.purge` grant does not cover —
    /// `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s grant, read
    /// before the program runs rather than at the door alone.
    ///
    /// `E0618` one class over, and asked under `E0618`'s conditions and no
    /// others: a literal queue name, a configuration this machine actually
    /// read, and the same grant list walked by the same
    /// `nvs_config::capability::Capabilities`. So it refuses nothing
    /// `nvs_runtime::capability::require` would have allowed, which is
    /// `rule:expressions/preparation-preserves-behaviour`'s earlier answer and
    /// never a different one; a computed name, or a check run with no
    /// configuration in front of it, says nothing and leaves the refusal to
    /// the door.
    ///
    /// The scope is a queue name matched exactly, not `db.open`'s host
    /// pattern: a queue name is a flat string the program itself picked, so
    /// there are no labels for a `*.` to match at and a grant that covered a
    /// prefix would cover queues nobody had named yet.
    ///
    /// `Core\Queue::delete` has no code of its own. Its queue arrives inside a
    /// `Queue\Id` at run time, so there is no written name to read and the
    /// door is the only place the question can be asked. In this band rather
    /// than the types one for `E0618`'s reason: what it reads is a grant, not
    /// a type.
    pub const E_UNGRANTED_QUEUE: Code = Code::new("E0637");

    /// `[http.client.tls] roots` naming no trust anchor at all — an empty
    /// list.
    ///
    /// Written out, that key is the whole answer to "whose certificates do
    /// you believe" for every outbound `https` call this process makes, and
    /// an empty list answers "nobody's". That is not a stricter deployment,
    /// it is one where every such call dies at the handshake with an unknown
    /// issuer — a message that sends an operator looking at the origin rather
    /// than at the key they wrote. Omitting the key is how a deployment asks
    /// for the compiled-in set, so there is nothing `[]` could have meant.
    pub const E_TLS_ROOTS_EMPTY: Code = Code::new("E0638");

    /// `[http.client.tls] min_version` naming a version this build does not
    /// speak.
    ///
    /// The client implements TLS 1.2 and 1.3 and nothing beneath them, so
    /// `"1.0"` and `"1.1"` are not floors it can be lowered to: accepting
    /// either would leave the floor at 1.2 while telling an operator they had
    /// chosen otherwise, which is the one outcome worse than refusing the
    /// key.
    pub const E_TLS_MIN_VERSION: Code = Code::new("E0639");

    /// `[http.client.tls] keylog` written on a host whose mode is
    /// `production`.
    ///
    /// The file it names collects every TLS session's secrets in the
    /// `SSLKEYLOGFILE` format, which is what decrypts this deployment's
    /// outbound traffic — the credentials inside it included — for anyone who
    /// can read the file. It is a debugging instrument, and the mode is where
    /// a deployment has already said whether it is debugging, so the key is
    /// announced in `development` and refused here rather than left for a
    /// reviewer to catch.
    pub const E_KEYLOG_IN_PRODUCTION: Code = Code::new("E0640");

    /// `[http.client.tls]` resolved, and the outbound client it describes
    /// could not be built from it — a `roots` entry that cannot be opened or
    /// holds no certificate, or a `keylog` whose file will not open.
    ///
    /// The keys are checked where they are read and the anchors are parsed
    /// where they are believed, so this is the second half arriving at boot:
    /// the block passed `E0638`–`E0640` and the files it names still did not
    /// yield a client. It is a refusal to start rather than a warning,
    /// because the alternative is a process that answers every outbound
    /// `https` call with a handshake failure an operator reads as the
    /// origin's fault. The message names the file.
    pub const E_TLS_CLIENT_UNBUILDABLE: Code = Code::new("E0641");

    /// A `[cache.process] fill_wait` that is not a wait — `0`, or `false`.
    ///
    /// `rule:concurrency/a-secret-fill-runs-once-per-process`
    /// : on a miss exactly one caller in the process runs the fill while
    /// every other caller waits for it, so this key is what bounds that
    /// wait. Zero makes every concurrent caller but one throw
    /// `TimeoutError` — the stampede's failure mode with the fetches
    /// removed, and nothing bought with it — and `false` asks for a wait
    /// nothing ends, which
    /// `rule:http-server/no-spelling-for-an-unbounded-wait` gives no
    /// spelling for anywhere else either. A caller that wants a shorter
    /// wait than the operator's writes `wait` at its own call site, where
    /// that decision is visible in review; an operator who wants a longer
    /// one raises this key.
    pub const E_FILL_WAIT_NOT_A_WAIT: Code = Code::new("E0642");

    /// A `[http.client.proxy]` block that writes no `resolve`.
    ///
    /// `rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`
    /// : the word is mandatory and there is no default, because both
    /// answers are commonly correct and either default silently does the
    /// wrong thing in somebody's production — `local` fails outright in a
    /// network where only the proxy resolves, and `proxy` gives up the
    /// address pin everywhere else. The refusal names both words rather
    /// than picking one, since the deployment's own network is what
    /// decides which is true and this file cannot know it.
    pub const E_PROXY_RESOLVE_MISSING: Code = Code::new("E0643");

    /// A `[http.client.proxy] resolve` that is neither `local` nor
    /// `proxy`.
    ///
    /// The two words are the whole roster, and they name who resolves the
    /// destination — which is the one question that decides whether
    /// `rule:security/net-address-policy` still sees an address. A third
    /// word has no reading to fall back on, and reading it as either of
    /// the two would pick a security posture out of a typo.
    pub const E_PROXY_RESOLVE_UNKNOWN: Code = Code::new("E0644");

    /// A `[http.client.proxy]` block whose `url` this client cannot dial —
    /// absent, not `http://`, naming no host, or carrying a credential or
    /// a port that is not one.
    ///
    /// `rule:http-server/an-outbound-proxy-is-operator-configured`
    /// : every destination is reached by `CONNECT` over plain TCP to this
    /// address, so `https://` is a second trust decision with no spelling
    /// here and every other scheme is one this client does not speak. A
    /// block with no `url` is refused rather than read as "no proxy":
    /// leaving the block out is how a deployment asks for that, and a
    /// written block that proxies nothing is a deployment believing its
    /// egress is tunnelled when it is not. Userinfo in the URL is refused
    /// for the same reason — `username` and `password` are where a
    /// credential is read from, so one written here would be dropped in
    /// silence.
    pub const E_PROXY_URL_UNDIALABLE: Code = Code::new("E0645");

    /// A `[http.client.proxy] bypass` entry that is not a host name — one
    /// with a port, a scheme, a `*` or a `/`.
    ///
    /// `rule:http-server/an-outbound-proxy-is-operator-configured`
    /// : the list matches the URL's host text, each entry exact or with a
    /// leading `.` for a suffix, and it is matched before anything is
    /// resolved. A range or a wildcard in that position hands back more
    /// than the operator can see they are handing back — the destinations
    /// it covers are the ones that then leave without the proxy — and a
    /// port or a scheme is an entry that can never match, which is a
    /// bypass an operator believes is in force. The refusal names the
    /// entry.
    pub const E_PROXY_BYPASS_ENTRY: Code = Code::new("E0646");

    /// A `[http.client.socket]` bound written with no bound in it — a
    /// `max_message` or a `send_timeout` of `false` or of zero.
    ///
    /// `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`
    /// : an outbound socket has no spelling for reassembling a message of any
    /// size or for waiting forever to write one, the same way
    /// `Core\Http\Options` has none for an unbounded call
    /// (`rule:http-server/no-spelling-for-an-unbounded-wait`). `false` removes
    /// a ceiling everywhere else in this file, and a socket whose message cap
    /// it removed is the memory of the task that opened it spent by whatever
    /// the peer decides to send. Zero is that value written the other way
    /// round: a cap of nothing admits no message and a wait of nothing writes
    /// no frame, so the block configured a socket that can hold no
    /// conversation. The refusal names the key and the two shipped values.
    pub const E_SOCKET_BOUND_REMOVED: Code = Code::new("E0647");

    /// A `[server] socket_mode` that is not a file mode — not octal, wider
    /// than `0777`, or carrying a bit that is not a permission.
    ///
    /// Refused rather than read as far as it parses, because the mode on a
    /// Unix-domain socket is the whole of who may connect to it and a
    /// connection that arrives is implicitly trusted for the forwarded
    /// headers (`rule:http-server/a-unix-socket-listener`). A value this
    /// reader had to guess the intent of would be a trust boundary chosen
    /// out of a typo, and the guess an operator would least expect is the
    /// one that widens it.
    pub const E_BAD_SOCKET_MODE: Code = Code::new("E0648");

    /// A `[server.connection]` bound written with no bound in it — any of
    /// that block's keys as `false` or as zero.
    ///
    /// `rule:concurrency/connection-bounds-are-finite` is the one bound set
    /// whose subject outlives the request that created it, so a connection
    /// held open with a ceiling an operator removed is memory and a
    /// descriptor nothing in the process ever reclaims. `false` removes a
    /// ceiling everywhere else in the tree
    /// (`rule:config/three-changeability-classes`) and has no meaning here;
    /// zero is the same value from the other side — no connection may be
    /// open, no frame may arrive and no wait may elapse — which is a server
    /// that upgrades nothing. The refusal names the key and the number the
    /// block ships with.
    ///
    /// A value that is not a size, a count or a duration at all is `E0601`
    /// in `nvs_config::value`'s own words; this code is only for a
    /// well-formed value whose *magnitude* is the problem.
    pub const E_CONNECTION_BOUND_REMOVED: Code = Code::new("E0649");

    /// `[http] csrf_key` holding something that is not a key: text that is
    /// not base64, or base64 of any length other than the one a key is.
    ///
    /// Refused rather than ignored, because ignoring it is the silent
    /// direction. The key is what arms the token half of
    /// `rule:security/csrf-is-on-by-default`, so a deployment whose value
    /// the door could not read would go on serving with its unsafe verbs
    /// unverified while its configuration says they are checked — the one
    /// way a security directive must not fail. The refusal names the key
    /// and what a key is, and never a byte of the value.
    pub const E_BAD_CSRF_KEY: Code = Code::new("E0650");

    // --- E07xx types, continued --------------------------------------------
    //
    // The E04xx band filled at `E0499`. Max-plus-one yields `E0500`, whose
    // band digits read as E05xx — IR and codegen — so that number is never
    // issued and the types band continues here instead. `docs/adr/README.md`
    // § *Decisions taken at project start* owns the reasoning; `tools/brief.py`
    // reports a filled band as full rather than handing out the number past
    // its end.
    /// An array element written through a root that is not a **place**:
    /// `$h->rows()["a"] = "y"`, `[1, 2]["0"] = "z"`, `($c ? $a : $b)["k"] = v`.
    ///
    /// `rule:types/arrays`'s copy-on-write separation has to be written back into
    /// whatever holds the array, and a temporary holds it nowhere — the write
    /// would land in a value dropped at the end of the statement. PHP 8.5
    /// accepts the spelling and discards the write with no diagnostic at all
    /// (checked with `php -r`, not assumed), which makes this a deliberate
    /// divergence (`rule:php-migration/an-element-write-needs-storage-to-write-back-into`)
    /// rather than a PHP-compatible refusal like `E0478` beside it.
    ///
    /// Parentheses are **not** a temporary: `($a)["0"] = "y"` writes `$a["0"]`
    /// here exactly as it does in PHP, because
    /// `nvs_syntax::ast::Expr::unparenthesized` is what finds the root.
    pub const E_ELEMENT_WRITE_ROOT_NOT_A_PLACE: Code = Code::new("E0700");
    /// A reference assignment, `$a = &$b;`.
    ///
    /// PHP binds the two names to one slot, so a later write through either
    /// is seen through the other. Novis has nowhere to put that: `rule:types/implicit-capture`
    /// removed by-reference capture, so no binding aliases another, and
    /// `rule:classes/two-copy-depths` fixes what a copy means, so the right-hand side is a copy at
    /// the point the assignment runs. The same reasoning already refuses
    /// `[&$x]` as [`E_ARRAY_ELEMENT_BY_REFERENCE`] — it is not a lowering
    /// that is missing, it is a thing the language does not have. `inout $x` at a
    /// *call site* stays, because a parameter's write-back is a copy in and a
    /// copy out rather than a shared slot.
    pub const E_ASSIGN_BY_REFERENCE: Code = Code::new("E0701");
    /// An argument binding an `inout` parameter, written without the marker.
    ///
    /// `rule:statements/inout-is-written-at-the-call` writes the word at both ends, and this is the half a
    /// rename alone would not have bought: `Adder::bump($n)` is otherwise
    /// indistinguishable at the point of call from `Adder::sum($a, $b)`,
    /// and only one of them writes to its caller's storage. The marker is
    /// deliberately not inference-assisted — a marker the compiler supplies
    /// is not a marker — so the omission is an error rather than a lint.
    pub const E_INOUT_ARG_MISSING: Code = Code::new("E0713");
    /// `inout` written at an argument that binds a by-value parameter, or at
    /// one that binds nothing a signature can name — a spread's entries, or
    /// any argument of a call through a `callable`.
    ///
    /// The mirror of [`E_INOUT_ARG_MISSING`]: a marker that is allowed to be
    /// wrong is worth nothing to the reader, so `rule:statements/inout-is-written-at-the-call` makes the extra
    /// one an error too. Through a `callable` it can never be right — ADR
    /// 0031 § 4 keeps that type opaque and [`E_CLOSURE_INOUT_PARAM`] refuses
    /// the declaration end outright — and a spread hands over a subject's
    /// entries rather than the subject, which is the same reason
    /// [`E_INOUT_ARG_NOT_A_PLACE`] wants one storage location. Through a
    /// **`mixed` receiver** it can never be right either, and for the reason
    /// the runtime dispatch refuses such a callee outright: an `inout`
    /// parameter list is packed and written back at the *call site*, which is
    /// the one thing a call whose callee is unknown until it runs cannot do.
    pub const E_INOUT_ARG_UNEXPECTED: Code = Code::new("E0714");
    /// `::class` written on a side that carries no class — `rule:types/class-constant`.
    ///
    /// `::class` answers the class the value *is*, so the operand has to carry
    /// one. An object does, and a `class<T>` does; `static::class` and
    /// `$obj::class` both lower (`rule:types/class-constant`), reading the name off a
    /// descriptor the frame already holds. What is left is the operand that
    /// might hold a class and might not, and the one that never can:
    ///
    /// * a `mixed` or a `?T` — accepting it would put a tag test and a throw
    ///   behind a spelling that reads like a member read. The narrowing that
    ///   lifts it is the one `->` already requires, and `Core\Reflect` (ADR
    ///   0019) is the door for a receiver whose type was genuinely erased.
    /// * a `class<T>` — already a descriptor, so its name is `rule:types/class-reference`'s
    ///   `as string` conversion rather than a member read.
    /// * anything else — a scalar, an `array<T>`, an enum: it never holds an
    ///   object at all.
    ///
    /// This is where Novis parts company with PHP, which accepts every operand
    /// and fails at run time on one that turns out not to be an object.
    ///
    /// A name that resolves to nothing is not this code: `Bogus::class` is
    /// [`E_UNDEFINED_CLASS`]'s `E0303`, the same mistake `new Undeclared()`
    /// takes, because `rule:types/class-constant`'s fold leaves it nowhere later to be
    /// caught.
    pub const E_CLASS_NAME_CONST_NOT_STATIC: Code = Code::new("E0702");
    // `E0703` is retired and is never reused: `spawn script` lowers through
    // `nvs-ir`, so there is nothing left for it to refuse.
    // `E0704` is retired and is never reused: `require` used for its
    // **value** is `rule:statements/a-require-expression-is-mixed`'s `mixed` and lowers, the site calling the
    // target file's own script frame and keeping what it hands back
    // (`nvs_ir::lower::Lowering::lower_expr`).
    /// `-`, `+` or `~` over an operand `rule:types/arithmetic`'s arithmetic table has no
    /// row for — a `string`, a `bytes`, an `array<T>`, a `bool`, `null`, a
    /// `callable` or an enum case. The sibling of [`E_INCREMENT_NOT_NUMERIC`]
    /// one operator over, and it exists for the same reason: PHP answers each
    /// of these by *converting* the operand first, and `rule:types/conversion` has no
    /// implicit conversion for that to be — so unary `+`, which is the
    /// identity over every numeric type, would otherwise be a silent identity
    /// over a `string` where PHP produces a number.
    ///
    /// An object takes `E_TYPE_MISMATCH` instead, one branch earlier in
    /// `nvs_types::expr::operators::reject_unary_arith_operand`: "Novis has no
    /// operator overloading" is the sentence that author needs, not "convert
    /// it first".
    pub const E_UNARY_ARITH_NOT_NUMERIC: Code = Code::new("E0705");
    /// `&`, `|`, `^`, `<<`, `>>` or `~` over an operand `rule:types/arithmetic`'s bitwise
    /// row has no entry for — that row is `int` and `uint` and nothing else,
    /// so a `float`, a `decimal`, a `string`, a `bool`, `null`, an `array<T>`,
    /// a `callable` or an object all take this code.
    ///
    /// The sibling of [`E_UNARY_ARITH_NOT_NUMERIC`] one row over, and the
    /// division of labour between them is the *numeric* operands: `-1.5` is
    /// arithmetic `rule:types/arithmetic` grants and `~1.5` is not, because a `float` and
    /// a `decimal` are numbers with no bit pattern to complement. A
    /// non-numeric operand of `~` keeps the unary code, whose sentence — "PHP
    /// converts this operand first" — is the one that author needs.
    ///
    /// Refusing is the only honest answer on offer: a bit-and over the `f64`'s
    /// own representation makes `1.5 & 1.5` answer `1.5`, where PHP answers
    /// the `int` `1`, and a `decimal` operand reaches no row of `nvs-ir`'s
    /// `rule:types/arithmetic` table at all.
    pub const E_BITWISE_NOT_INTEGER: Code = Code::new("E0706");
    /// A `bytes`, an `array<T>`, an enum case or a `void` call used where a
    /// `string` is produced *implicitly* — `.`, `.=`, an interpolated piece,
    /// `echo`/`print`. `rule:types/conversion`'s "anything → `string`" row is "total for
    /// scalars; an object needs `Stringable`", and these are the types it
    /// does not reach at all.
    ///
    /// The explicit `as string` is deliberately not this code's business:
    /// `rule:types/conversion` grants `bytes as string`, and the whole of the difference
    /// is an encoding decision made out loud rather than by a `.` operator. An
    /// object with no `toString` keeps [`E_STRINGABLE_REQUIRED`], whose
    /// sentence names the interface to implement.
    ///
    /// `null` is **not** here: it renders as the empty string, which is both
    /// PHP's answer and the one a `?string` holding `null` already gets at run
    /// time — `nvs_ir::lower::expr`'s `concat_operand` owns that row.
    pub const E_NO_STRING_FORM: Code = Code::new("E0707");
    /// An `expr as T` whose operand and target name no row of `rule:types/conversion`'s
    /// conversion table, nor of the ADRs that table delegates rows to —
    /// `rule:types/conversion`'s `string` ↔ `bytes` pair, `rule:types/conversion`'s `decimal` ones
    /// and `rule:types/conversion`'s enum ones.
    ///
    /// The table is *closed*: `as` "either produces a value of the target type
    /// or throws", so a pair with no row has nothing to produce and nothing to
    /// throw. `true as int`, `$xs as string`, `$i as bytes`, `$case as float`
    /// and `$obj as OtherClass` are the shapes that reach it, and each help
    /// names the spelling that says what was meant instead.
    ///
    /// A class target is this code's, not [`E_CLASS_CONVERSION_TARGET`]'s:
    /// that one is `rule:expressions/nullable-conversion-availability`'s *written* `as ?T` sugar, and a plain
    /// `as SomeClass` is the missing row rather than the withdrawn parse
    /// roster. The two never fire on the same expression.
    pub const E_NO_CONVERSION: Code = Code::new("E0708");
    /// An `expr as ?T` whose row cannot fail, which `rule:expressions/nullable-conversion-availability` makes a
    /// compile error naming `as T`.
    ///
    /// `as ?T` "yields `null` exactly where `as T` would throw" — so over a
    /// row that never throws it promises a `null` no run can produce, and
    /// every reader after it is forced to check for it. `$i as ?int` (the
    /// identity), `$i as ?string` (`rule:types/conversion`'s total "anything →
    /// `string`" row), `$x as ?bool` (`rule:expressions/truthy-positions`'s, which has an answer for
    /// every type) and `Mode::Read as ?Mode` are the shapes that reach it.
    ///
    /// The sibling refusals are the other rows of that same table:
    /// [`E_NO_CONVERSION`] for a pair naming no row at all, asked of the `T`
    /// inside the sugar, and [`E_CLASS_CONVERSION_TARGET`] for a class
    /// target. No two of them ever fire on the same expression.
    pub const E_NULLABLE_CONVERSION_CANNOT_FAIL: Code = Code::new("E0709");
    /// A `Core`-owned class rendered as text where the spec gives it no
    /// `toString` — an `echo`, an interpolation, a `.` operand or an
    /// `as string`.
    ///
    /// `rule:classes/stringable` makes `Stringable` the one way an object renders, and a
    /// `Core` class does not reach that rule the way a user class does: it
    /// declares no interfaces, its members being `nvs_stdlib::registry`'s
    /// rows, so that registry is the one home for which `Core` classes render
    /// and this diagnostic is that answer read back at the site. Without it a
    /// miss falls through to a runtime dispatch that cannot see a native
    /// member at all.
    ///
    /// The sibling for a user class is [`E_STRINGABLE_REQUIRED`], which asks
    /// the class graph the same question; the two never fire together,
    /// because a class is `Core`-owned or it is not.
    pub const E_CORE_CLASS_NOT_STRINGABLE: Code = Code::new("E0710");
    /// An `expr as T` into an object target that names **no testable class** —
    /// plain `object`, a shape, `callable`, or a `Core`-owned class — from an
    /// operand that is not already an object.
    ///
    /// `rule:types/conversion` tabulates no row producing an object, and the one reason a
    /// class target is admitted at all is that it can be *checked*: the
    /// downcast out of `mixed` tests the value's runtime class and throws when
    /// it misses. `object`, a shape and a `callable` name no class at all, so
    /// there is no descriptor to test against. A `Core` class is refused for a
    /// narrower reason: its descriptor is the process's rather than the unit's,
    /// and the downcast resolves its target through
    /// `nvs_codegen`'s `class_desc_const`, which answers only for a class the
    /// unit built — while `$v instanceof Core\Time\Date` reaches the same
    /// descriptor as an imported symbol and does test against it. Either way
    /// the conversion could only *assert* the tag it cannot verify, and the
    /// honest answer is a diagnostic where it is written.
    ///
    /// An operand that is already an object is untouched and is the free
    /// widening row: `$plain as object` runs nothing, because both sides are
    /// one pointer.
    ///
    /// The sibling for a target that *does* name a class is
    /// [`E_NO_CONVERSION`], which refuses the pair sharing no value at all;
    /// the two never fire together, because a target names a testable class or
    /// it does not.
    pub const E_UNTESTABLE_CONVERSION_TARGET: Code = Code::new("E0711");

    /// A `name:` argument at a call through a `callable`.
    ///
    /// `rule:types/closure-literal` gives `callable` no parameter list — it is one opaque type
    /// whatever closure a variable holds — so there is no parameter for a name
    /// to fill, at the site or below it: a closure value records its arity and
    /// its parameter *tags*, never their names, so nothing at run time could
    /// resolve one either. PHP can only allow it because a `Closure` there
    /// carries its declaration.
    ///
    /// The same rule as [`E_CLOSURE_INOUT_PARAM`], read from the call site's
    /// end rather than the literal's. It has no sibling at a *resolved*
    /// target: every signature the checker builds names its parameters (ADR
    /// 0063 R2), so a `name:` there is a spelling question and never an
    /// absence — [`E_UNKNOWN_ARG_NAME`] is the whole of it. A `...`
    /// argument is not refused here: how many arguments it hands over is its
    /// own run-time length, which needs no parameter list to be meaningful.
    ///
    /// A call through a **`mixed` receiver** takes the same code, because it
    /// is the same absence: `rule:types/erased-member-access` defers that call to the receiver's
    /// runtime class, whose method row carries the callee's arity and
    /// parameter tags and — for a closure value's reason — never its parameter
    /// names.
    pub const E_NAMED_ARG_THROUGH_CALLABLE: Code = Code::new("E0712");

    /// `<`, `<=`, `>`, `>=` or `<=>` over an operand `rule:types/arithmetic` gives no
    /// ordering row for.
    ///
    /// That table orders the numeric types against each other and, through
    /// `rule:classes/comparable`, two objects of one class that implements `Comparable`. It
    /// is a *closed* list, and everything else PHP orders it orders by
    /// converting first — which Novis never does by itself. A `string`, a
    /// `bytes`, an `array<T>`, a `callable`, an enum case and `null` therefore
    /// have no `<` at all, and each help names the member that does say what
    /// was meant: `Core\Str::compare` for text, `as int` for an enum case.
    ///
    /// The object family keeps [`E_COMPARISON_REQUIRES_COMPARABLE`] rather
    /// than joining this code, so that "these two do not order" is one
    /// diagnostic however the receiver was spelled.
    ///
    /// `bool` is deliberately *not* refused: `false < true` is the machine
    /// ordering of the one bit, it is PHP's answer as well, and it needs no
    /// conversion to be exact — the row is left out of `rule:types/arithmetic`'s table
    /// because that table is about the numeric widenings, not because two
    /// `bool`s are unordered.
    pub const E_ORDERING_HAS_NO_ROW: Code = Code::new("E0715");

    /// `+`, `-`, `*`, `/`, `%` or `**` over an operand `rule:types/arithmetic` gives no
    /// arithmetic row for.
    ///
    /// That table's operands are the numeric types — `int`, `uint`, `float`
    /// and, through `rule:types/arithmetic`, `decimal` — and it is as *closed* as the
    /// ordering row [`E_ORDERING_HAS_NO_ROW`] refuses against. Everything else
    /// PHP adds it adds by converting first, which `rule:types/conversion` never does by
    /// itself, so a `bool`, a `string`, a `bytes`, an `array<T>`, a
    /// `callable`, `null` and an object have no `+` at all and each help names
    /// the spelling that says what was meant.
    ///
    /// `bool` is the operand this code exists for, and it is the reverse of
    /// [`E_ORDERING_HAS_NO_ROW`]'s own `bool` exemption: two `bool`s *order*
    /// exactly as the one bit they already are, but adding them is PHP's
    /// "convert to `int` first" and nothing else — and left unrefused it would
    /// not even answer PHP's number, `nvs-codegen` reading `true + true` as an
    /// `iadd` over the `i8` a `bool` is stored in. An enum case keeps
    /// [`E_ENUM_ARITHMETIC_UNSUPPORTED`], so "this operand has no arithmetic"
    /// reads as one diagnostic per rule rather than per type.
    pub const E_ARITHMETIC_HAS_NO_ROW: Code = Code::new("E0716");

    /// `%` with a `float` operand.
    ///
    /// The one row this band refuses that both operands *are* numbers for.
    /// `rule:types/arithmetic`'s "either operand a `float`" row is written for the
    /// arithmetic operators as a family, but `%` is the one member of it PHP
    /// does not answer that way: PHP converts both operands to an integer and
    /// returns an integer, where the row would return a `float`. The two are
    /// different answers for `7.5 % 2`, no ADR settles which of them Novis
    /// gives, and `as` is one character away — so the operator is refused at
    /// **both** ends rather than guessed at either. The other end is the
    /// tagged one, `nvs_runtime::helpers::value_arith`, where the same rule
    /// arrives as a catchable throw because only the runtime tags can see it.
    ///
    /// `decimal % float` is [`E_DECIMAL_FLOAT_ARITHMETIC`] instead: that pair
    /// has no common arithmetic type at all, which is the earlier objection.
    pub const E_FLOAT_MODULO: Code = Code::new("E0717");
    /// A call that returns `void` used as an operator's operand.
    ///
    /// Every other refusal in this band is "this type names no row of
    /// `rule:types/arithmetic`'s table". This one is a step earlier: a `void` call has no
    /// value *at all*, so there is no operand for a row to be about, and the
    /// question of which row applies never arises. The two ends of the
    /// language agree on nothing here — `nvs-ir` has no representation to
    /// lower and `nvs-codegen` no machine type to emit — so it is refused
    /// where it is written rather than reaching either.
    ///
    /// Deliberately not the `.` operator's, which keeps
    /// [`E_NO_STRING_FORM`]: that code's roster already names a `void` call
    /// among the types with no implicit `string` form, and it is the
    /// wording an author who wrote `echo` or an interpolated piece needs. One
    /// rule, one code, both ways round.
    ///
    /// A `void` call in a *condition* is neither of these two and takes
    /// [`E_VOID_IS_NOT_A_CONDITION`] instead.
    pub const E_VOID_IS_NOT_AN_OPERAND: Code = Code::new("E0718");
    /// A call that returns `void` tested for truth.
    ///
    /// `rule:expressions/truthy-positions` makes a condition the one place a value is tested without
    /// `as`, and "a value" is exactly what a `void` call is not — so its
    /// truthy table, like `rule:types/arithmetic`'s, has nothing to look a row up for.
    /// The two refusals are one sentence apart and are deliberately two
    /// codes: [`E_VOID_IS_NOT_AN_OPERAND`] is read by an author who wrote an
    /// operator and reads "not an operand", which is the wrong sentence for
    /// `if (V::nothing())`, where no operator is written at all.
    ///
    /// The line between them is *which table has no row*, not which syntax
    /// was used. `&&`, `||` and `??` are `rule:types/arithmetic`'s operands and keep
    /// [`E_VOID_IS_NOT_AN_OPERAND`]; `!` and `empty()` are `rule:expressions/truthy-table`'s
    /// truthy test written out and take this one, alongside the four
    /// statement conditions and a ternary's.
    ///
    /// Unrefused, this reaches no diagnostic and no answer either: `nvs-ir`
    /// lowers a `void` call to no value, so its truthy slice panics on a
    /// representation the table has no row for, naming a bug in the compiler
    /// for what is a mistake in the program.
    pub const E_VOID_IS_NOT_A_CONDITION: Code = Code::new("E0719");
    /// An `implements I by $field;` clause whose `$field` cannot answer `I` —
    /// `rule:classes/delegation-by-field` bullet 1.
    ///
    /// The delegate has to be a **declared property** of the class, whose
    /// type is a **non-nullable** class or interface that itself satisfies
    /// the delegated interface. Each half of that is a real failure the
    /// synthesized forward has no answer for: a name that is no property has
    /// no slot to read, a nullable one has nothing to dispatch on when it
    /// holds `null`, and a type that does not satisfy `I` has no member for
    /// the forward to name.
    ///
    /// This code is what makes `crate::conformance`'s check **per member**
    /// rather than whole-class. Without it, a member no forward covers could
    /// not be told from one whose field cannot answer it, so a class with
    /// any `by $field` clause at all would be exempt from
    /// [`E_INTERFACE_METHOD_MISSING`] entirely; with it, the field is judged
    /// here and every member the delegation does not supply is judged there.
    pub const E_DELEGATE_TYPE_MISMATCH: Code = Code::new("E0720");
    /// A member of a delegated interface whose shape `rule:classes/delegation-by-field`'s
    /// synthesized forward cannot express: a `static` member, a variadic
    /// parameter list, or an `inout` parameter.
    ///
    /// Each is a limit of this compiler rather than a rule of the language,
    /// and each has the same cause — the forward is a whole method whose body
    /// passes its parameters straight on. A `static` member has no receiver to
    /// read the field off (`rule:statements/static-is-a-member-modifier` gives class storage none), and a variadic
    /// or `inout` list is packed and written back at the *call site*
    /// (`rule:statements/inout-is-written-at-the-call`), so passing it on would pack it twice.
    ///
    /// It is a diagnostic where the clause is written because the alternative
    /// is silent: no forward is synthesized, the class is exempt from
    /// [`E_INTERFACE_METHOD_MISSING`] anyway, and the call lands on
    /// `nvs_runtime::nvs_abstract_method` — a `FATAL` naming a compiler bug
    /// for a program the front end accepted. The help names the way out:
    /// write the member on the class by hand, which § 4 already allows and
    /// which the forward would have lost to.
    pub const E_DELEGATE_MEMBER_NOT_FORWARDABLE: Code = Code::new("E0721");
    /// A visibility keyword on a parameter of a method that is not the
    /// `constructor` — `rule:classes/delegation-by-field`'s own backlog line.
    ///
    /// `public`/`protected`/`private` on a parameter is PHP 8's constructor
    /// promotion and nothing else: it says where a **property** may be read
    /// from, and only a constructor declares one.
    /// `nvs_syntax::ast::Param::is_promoted` is the one home of which
    /// parameters promote, and `crate::signatures::record_promoted_properties`
    /// only ever asks it of a constructor — so the keyword written anywhere
    /// else declares nothing and gives no slot, where PHP refuses it
    /// outright.
    ///
    /// It is a diagnostic rather than a widening of promotion because an
    /// ordinary method has no allocation to promote *into*: a property is a
    /// slot on an instance, armed once at `new`, and a method that may be
    /// called any number of times has no such moment.
    pub const E_PROMOTED_PARAM_OUTSIDE_CONSTRUCTOR: Code = Code::new("E0722");
    /// A `foreach` key binding over an `array<T>` declared as anything but
    /// `string` — `rule:types/arrays`'s "every key is a `string`" read at the one
    /// place a program can name a key's type.
    ///
    /// An array has exactly one stored key type, so `foreach ($a as int $k
    /// => …)` is not a narrowing the checker cannot prove: it is always
    /// wrong. It is a separate code from [`E_TYPE_MISMATCH`] because there is
    /// no *value* being assigned here for the two types to disagree about —
    /// the subject's own container fixes the answer — and the help therefore
    /// names the rule rather than the pair.
    ///
    /// Unrefused it reaches no diagnostic *and* no answer: `nvs-ir` lowers a
    /// key binding only at `string` (`rule:types/arrays` again, one crate down) and
    /// asserts on anything else, so a mistake in the program surfaces as a
    /// panic naming a compiler gap. A subscript's `$a[8]` is normalised to
    /// `$a["8"]` at the subscript rather than converted, and there is no
    /// matching normalisation on the way *out* of a `foreach` — which is why
    /// the binding is refused instead of being given the conversion.
    pub const E_FOREACH_KEY_TY: Code = Code::new("E0723");

    /// `rule:security/secret-sinks-refuse`'s debug-dump sink, as
    /// `rule:errors/record-transformations`'s redaction row states it: a `secret`-qualified value written at a
    /// `Core\Debug::dump`/`render` call site is refused where it is written.
    ///
    /// It is a *call-site* rule rather than a parameter type, for ADR 0033
    /// § *Context*'s reason and `Core\Log::write`'s: both members declare
    /// `mixed`, which a `secret string` satisfies, so the only place the
    /// qualifier is still visible is the argument expression itself.
    ///
    /// A separate code from [`E_SECRET_THROWABLE_MESSAGE`] because the
    /// disclosure is a different one — a dump goes to the diagnostic channel
    /// a person reads, not into a message a program carries — and because
    /// this one names the redaction that *does* apply, one storage kind
    /// along: a `secret`-typed **property** of a dumped object is a Redacted
    /// node rather than a refusal, so the help distinguishes the value the
    /// author handed over from the value a record redacts for them.
    pub const E_SECRET_DEBUG_ARGUMENT: Code = Code::new("E0724");
    /// An attribute payload's field value is not a compile-time constant —
    /// `rule:attributes/payload-is-a-compile-time-constant`. The whole literal is resolved once, at compile time, into the
    /// unit's constant pool, the same storage class an enum case's backing
    /// value already uses; there is no "evaluate this attribute's arguments"
    /// step at class-definition time for a variable, a call or a `new` to be
    /// evaluated *in*, which is what PHP's lazily-constructed attribute
    /// object has and this one deliberately does not.
    pub const E_ATTRIBUTE_VALUE_NOT_CONSTANT: Code = Code::new("E0725");
    /// The named form of an attribute names something that is not a
    /// shape-typed `type` alias —
    /// `rule:attributes/attach-sites-and-forms`. `Name` there is never a class and never a new namespace of
    /// attribute kinds: it is a pre-existing alias whose right-hand side is a
    /// shape, and its whole job is to be the type the attached literal is
    /// checked against.
    ///
    /// A name nothing declared at all takes [`E_UNDEFINED_CLASS`] instead,
    /// exactly as any other unresolvable name does — an attribute name is not
    /// a second namespace, so it gets no "no such attribute" of its own. This
    /// code is for a name that *does* denote something and denotes the wrong
    /// thing: a class (the spelling PHP's attributes would have instantiated),
    /// an interface, an enum, or a `type` alias for something that is not a
    /// shape.
    pub const E_ATTRIBUTE_NAME_NOT_A_SHAPE: Code = Code::new("E0726");
    /// A `secret`-qualified class constant reaches an attribute payload —
    /// `rule:security/secret-sinks-refuse`
    /// 's attribute-payload sink, the one that exists *because* of
    /// `rule:attributes/payload-is-a-compile-time-constant`: a payload holds only compile-time constants, and a class constant
    /// is one of them, so the qualifier's own storage class is the only way a
    /// `secret` value could get in there at all.
    ///
    /// It is a sink rather than a mismatch because of where the value ends up:
    /// the payload is folded into the compiled unit's constant pool and handed
    /// back by § 4-5's retrieval to anything that asks, so the credential is
    /// written into the program's own metadata rather than read at run time
    /// from somewhere a capability guards.
    ///
    /// A separate code from [`E_SECRET_DEBUG_ARGUMENT`] because the way out is
    /// a different one. That sink has `Core\Secret::reveal()` in front of it;
    /// this one has nothing, `reveal` being a call and a payload admitting
    /// none — so the help names the fix that exists, which is to keep the
    /// secret out of the metadata entirely.
    pub const E_SECRET_ATTRIBUTE_PAYLOAD: Code = Code::new("E0727");
    /// A `Core\Attributes::get<T>` whose target carries more than one attached
    /// literal satisfying `T` — `rule:attributes/retrieval-folds-while-checking`.
    ///
    /// A declaration's attached-attribute list is fully static, so "which one
    /// did I get?" is a question this compiler answers rather than one a test
    /// run answers. `::all<T>` is the member that wants every match, and the
    /// help names it.
    pub const E_ATTRIBUTE_RETRIEVAL_AMBIGUOUS: Code = Code::new("E0728");
    /// The `<T>` written at a `Core\Attributes::get`/`all` call site is not a
    /// shape type — `rule:attributes/structural-retrieval`.
    ///
    /// Retrieval is *structural*: `T` is what an attached literal is matched
    /// against under `rule:types/shape-type`'s width subtyping, so a `T` that is not a
    /// shape names nothing an attribute payload could ever satisfy.
    pub const E_ATTRIBUTE_TYPE_ARG_NOT_A_SHAPE: Code = Code::new("E0729");
    /// The `$target` of a `Core\Attributes::get`/`all` call does not name a
    /// declaration this unit holds — `rule:attributes/structural-retrieval`.
    ///
    /// § 4 fixes the spelling: a method (a constructor included) is named by
    /// its own first-class-callable reference `Foo::bar(...)`, and a class by
    /// its `constructor`'s. Retrieval is resolved entirely at compile time, so
    /// the reference is *inspected* where it is written rather than evaluated
    /// — anything else has no declaration to read an attribute list off.
    pub const E_ATTRIBUTE_TARGET_NOT_A_DECLARATION: Code = Code::new("E0730");
    /// An attached literal satisfies the `T` a `Core\Attributes` retrieval
    /// asked for, but holds a value this compiler cannot materialize.
    ///
    /// `rule:attributes/retrieval-folds-while-checking` replaces the call with the payload itself, so every value
    /// in a matched payload has to have a constant form. Every spelling § 2
    /// admits has one — the payload is folded under the scope it was
    /// *written* in, so a class constant, `Foo::class` and an enum case each
    /// resolve to the value a read of the same name inlines
    /// (`nvs_types::retrieval`'s own docs own that). What is left is the
    /// residue: a class constant whose *own* declaration folded to no value,
    /// which is [`E_CLASS_CONST_NO_CONSTANT_FORM`]'s gap surfacing at the one
    /// site that needs the value rather than the name.
    pub const E_ATTRIBUTE_PAYLOAD_UNFOLDABLE: Code = Code::new("E0731");
    /// The first-class callable spelling `$m->method(...)` written on a
    /// `mixed` receiver.
    ///
    /// Every other erased receiver takes [`E_METHOD_ON_ERASED_RECEIVER`] for
    /// the whole call; `mixed` is `rule:types/conversion`'s one unchecked position and
    /// defers instead, so a *call* through it dispatches on the receiver's
    /// runtime class. This spelling does not call at all: `rule:types/callable-is-a-closure` makes it a
    /// closure **value**, which carries the callee's arity and parameter tags
    /// in the value itself (`nvs_runtime::closure`), and there is no class
    /// here to read either off — the receiver's descriptor answers a call it
    /// is present at, not a value that outlives the site. Narrowing the
    /// receiver, or calling the member directly, is the fix; both are what the
    /// help names.
    pub const E_FIRST_CLASS_CALLABLE_ERASED_RECEIVER: Code = Code::new("E0732");
    /// A `#[Test]` method whose declaration is not the shape `rule:testing/test-attribute`
    /// requires: `static`, not `public`, or returning anything but `void`.
    ///
    /// One code for the family rather than three, because it is one question —
    /// what shape a test method must have — and the runner asks it once: it
    /// constructs the class and calls the member with no arguments and no
    /// result, so a `static` member has no receiver for a `#[Fixture]` to be
    /// installed on, a non-`public` one cannot be called from outside the
    /// class at all, and a returned value has nowhere to go and nothing that
    /// would look at it. Each help names the modifier or the annotation to
    /// change.
    ///
    /// The fourth error in that bullet — two `#[Test]` methods with one name —
    /// is [`E_DUPLICATE_DECLARATION`] instead, being the same mistake the
    /// option written twice already draws, and the fifth (`skip: true`) is the
    /// option roster's own type check.
    pub const E_TEST_METHOD_SHAPE: Code = Code::new("E0733");
    /// A `#[Test(retries: n)]` with no `because:` beside it.
    ///
    /// `rule:testing/runner-is-strict` grants retries and charges a written reason for them in
    /// the same sentence: a retry that nobody had to justify is how a suite
    /// stops noticing that it is unreliable, and the reason is what a reader
    /// of the attribute has instead of the run that produced it. It is a
    /// separate code from the option roster's own type check because nothing
    /// about `retries: 2` is ill-typed — the mistake is the option that is
    /// *absent*, which is the one thing an all-optional bag cannot say.
    pub const E_TEST_RETRIES_WITHOUT_REASON: Code = Code::new("E0734");
    /// A `#[Fixture]` method the runner could not build a value from.
    ///
    /// `rule:testing/fixtures` builds a fixture **once, in the parent isolate**, and
    /// injects the value it returns into each test that declares a parameter
    /// of its type — so a fixture that is not `static` has no instance to be
    /// built against (§ 20 gives each test its own, which is the opposite of
    /// once), a non-`public` one cannot be called from outside its class, and
    /// one returning `void` produces nothing for a parameter to take. A
    /// method carrying both markers is the fourth: `#[Test]` requires exactly
    /// what this refuses, so it is one method claiming to be two things.
    ///
    /// It is a separate code from [`E_TEST_METHOD_SHAPE`] rather than a
    /// widening of it because the two rules are inverted — a test is an
    /// instance method returning nothing, a fixture a `static` one returning
    /// something — so a single code would have to word its help both ways.
    /// Two fixtures of one class returning one type is neither, and draws
    /// [`E_DUPLICATE_DECLARATION`]: § 8 resolves by type, so it is one
    /// declaration made twice.
    pub const E_FIXTURE_METHOD_SHAPE: Code = Code::new("E0735");
    /// A `#[Test]` or `#[Fixture]` parameter whose type no `#[Fixture]` of the
    /// class supplies.
    ///
    /// `rule:testing/fixtures` resolves a parameter **by type**, while compiling, so that
    /// "an unsatisfiable parameter is a diagnostic rather than a null at
    /// runtime" — this is that diagnostic, and it is the `Widget` line of that
    /// section's own worked example. One code covers a test's parameter and a
    /// fixture's own, because it is one question asked of one roster: what
    /// supplies this type. § 9's data rows are the other answer, and they
    /// fill a parameter **by name**, so this fires only where neither roster
    /// reaches it — which is why the help names both.
    pub const E_FIXTURE_PARAMETER_UNSUPPLIED: Code = Code::new("E0736");
    /// A `#[Fixture]` that requires itself, directly or through others.
    ///
    /// `rule:testing/fixtures`'s last sentence: a fixture may declare fixture parameters
    /// of its own, and a cycle among them is a compile error. It is separate
    /// from [`E_FIXTURE_PARAMETER_UNSUPPLIED`] because every parameter on such
    /// a cycle *is* supplied — by a roster that cannot be built in any order,
    /// which is a fact about the chain rather than about any one parameter.
    pub const E_FIXTURE_CYCLE: Code = Code::new("E0737");
    /// A `#[TestWith(...)]` data row that does not describe the method it is
    /// attached to.
    ///
    /// `rule:testing/data-rows` matches a row's shape literal against the method's
    /// parameters **by name and by type**, so one code covers every way the
    /// two can fail to line up: a field naming no parameter, a field whose
    /// value is not a literal of that parameter's declared type, a row that
    /// leaves out a field its siblings supply — every row of one method fills
    /// the same parameters, because the parameters a method declares do not
    /// vary row by row — and the marker written on a method that is no
    /// `#[Test]` at all.
    ///
    /// It is one code because it is one question — does this row describe
    /// this method — asked once per row, and because the fix is the same
    /// every time: write the row against the parameter list. A parameter
    /// **no** row names is not this code but
    /// [`E_FIXTURE_PARAMETER_UNSUPPLIED`], that being a question about the
    /// two rosters rather than about a row.
    pub const E_TEST_ROW_FIELD: Code = Code::new("E0738");

    /// A method declaring a return type other than `void` has a path that
    /// reaches the end of its body without returning or throwing.
    ///
    /// `rule:types/declaration`'s "nothing is untyped" has no answer for what such a path
    /// hands back: `nvs_ir::lower::lower_method` seals a body's fall-through
    /// exit with `Terminator::Return(None)`, so the caller reads a value of no
    /// declared type at all where an `int` was promised. PHP returns `null`
    /// there; Novis has no implicit `null` for a non-nullable declaration and
    /// will not invent one, so the path is refused where it is written.
    ///
    /// The analysis behind it is `nvs_types::returns`, and it is deliberately
    /// asymmetric: every shape it cannot prove *falls through* is treated as
    /// exiting, so an unusual body is accepted rather than wrongly refused.
    pub const E_MISSING_RETURN: Code = Code::new("E0739");
    /// The first-class callable spelling written on `new`: `new C(...)`.
    ///
    /// `rule:types/callable-is-a-closure`'s kept list is a list of *members* — `Class::method(...)`,
    /// `$obj->method(...)`, `self`/`static`/`parent::method(...)` — and a
    /// constructor is not one of them: `new` names a class, and the closure
    /// this syntax builds carries a callee, not an allocation. PHP refuses it
    /// for the same reason ("cannot create Closure for new expression"), so
    /// refusing it is the PHP-compatible answer as well as the only one with a
    /// meaning. `fn (): C => new C(…)` is the closure that was wanted, and it
    /// is what the help names.
    ///
    /// Separate from [`E_FIRST_CLASS_CALLABLE_ERASED_RECEIVER`] because the
    /// two are opposite failures: that one resolves a member and has no class
    /// to bind it to, this one names a class and has no member.
    pub const E_FIRST_CLASS_CALLABLE_NEW: Code = Code::new("E0740");

    /// A body declaring `static` returns a value that is not the called class.
    ///
    /// `rule:statements/static-is-a-member-modifier`'s late static binding makes `static` mean *the class the
    /// call named*, which a subclass may be — so `Base::make(): static` read
    /// through `Leaf::make()` promises a `Leaf`. A body that answers
    /// `new self()` keeps that promise only when nobody ever extends `Base`,
    /// and PHP catches the rest at run time with a `TypeError`. Novis has no
    /// such check below the type system, so the promise is held at the
    /// declaration instead: a `return` in a body declaring `static` must be
    /// `$this`, `new static(...)`, or a call forwarded through
    /// `static`/`self`/`parent`/`$this` to a member that itself returns
    /// `static`. `nvs_types::signatures::MethodSig::returns_static` owns the
    /// reasoning and what the alternative would have cost.
    ///
    /// Refusing is stricter than PHP, which accepts the declaration and only
    /// errors at a subclass call site. That is AGENTS.md's priority ordering
    /// applied as written: a refused program has no observable behaviour to be
    /// incompatible with, while an accepted one hands a `Leaf`-typed binding a
    /// `Base` and every later check reads a class label that was never there.
    /// `self` is the return type that was meant when the body really does
    /// answer the declaring class, and it is what the help names.
    pub const E_STATIC_RETURN_NOT_CALLED_CLASS: Code = Code::new("E0741");

    /// A parameter declares `void` or `never`.
    ///
    /// `rule:types/grammar` says both are return-only, and there is nothing else they
    /// could mean in an argument position: `void` is the absence of a value,
    /// so no argument satisfies it, and `never` is the empty type, so no
    /// argument satisfies that either. A method declaring one has no callable
    /// arity — every call site is refused for a reason phrased about the
    /// argument rather than about the declaration that made it impossible.
    ///
    /// Refused at the declaration for that reason, in the one pass that reads
    /// every method signature (`nvs_types::signatures`), so an abstract method
    /// and an interface signature are covered as well as a body. Unrefused,
    /// both spellings type-check and reach the back end: `never` panics
    /// `nvs_ir::lower`'s representation map and `void` lowers and dies in
    /// `nvs-codegen` reading a value of representation `void` — internal
    /// errors for one declaration nothing can call.
    pub const E_VOID_OR_NEVER_PARAMETER: Code = Code::new("E0742");

    /// `Core\Program::implementing<T>()`'s type argument is not an interface.
    ///
    /// `rule:programs/implementing` writes the constraint into the signature itself — "`T` must
    /// be an interface type" — and the ADR says why it is not a convenience:
    /// the interface is what gives `$module->register($this)` a static type,
    /// where [`E_METHOD_ON_ERASED_RECEIVER`]'s `object` is opaque and a shape
    /// describes data rather than methods. A class, an enum or a scalar written
    /// here would enumerate something, but nothing could then be *called* on
    /// what came back, so the answer would be an array nobody can use.
    ///
    /// Reported where the type argument is written, in the same pass that
    /// expands the call, because the expansion needs the answer anyway.
    pub const E_PROGRAM_TYPE_ARG_NOT_AN_INTERFACE: Code = Code::new("E0743");

    /// A class the enumeration would instantiate declares a constructor that
    /// takes arguments.
    ///
    /// `rule:programs/implementing`: "Each such class needs a no-argument constructor; a
    /// diagnostic names any that does not, and dependencies arrive through the
    /// interface's own methods instead." The call expands to one `new`
    /// expression per implementor and there is no call site to write arguments
    /// at, so this is refused rather than defaulted — a constructor parameter
    /// with a default would otherwise make the enumeration silently pick it.
    ///
    /// The primary span is the *call*, not the offending declaration: the
    /// program being compiled is the one that asked for the enumeration, and
    /// the implementing class may be in a file this program only reached
    /// through § 3's scan. The class is named in the message and in the help.
    pub const E_PROGRAM_IMPLEMENTOR_NEEDS_NO_ARGUMENT_CONSTRUCTOR: Code = Code::new("E0744");

    /// Two `#[Option]`s of one `#[Command]` claim the same short or long
    /// spelling.
    ///
    /// `rule:tooling/commands-are-compiled`'s second compile error, and one of the two that need no
    /// table: a command line is matched by the spellings one method's
    /// parameter list declares, so two parameters answering to `-n` is
    /// decidable from that list alone. An argument parser that discovers this
    /// at run time either takes the first match or the last, and both are a
    /// silent wrong answer to `-n`.
    ///
    /// The spelling compared is the *effective* one: a parameter's own name is
    /// its long spelling unless `long:` gives another, which is why
    /// `#[Option(long: "dryRun")] bool $force` collides with a plain
    /// `#[Option] bool $dryRun` beside it.
    pub const E_OPTION_SPELLING_TAKEN: Code = Code::new("E0745");

    /// An `#[Option]` is attached to a parameter whose declared type has no
    /// conversion from `string`.
    ///
    /// `rule:tooling/commands-are-compiled`'s third compile error. A matched value's type comes from
    /// the parameter and the argument arrives as text, so a parameter no text
    /// can be converted into is an option that could never be given — and § 6
    /// takes `rule:security/route-capture-is-laundered-by-its-type`'s conversion roster unchanged, which is why an
    /// `array<int>`, a shape or a class other than `Core\Uuid` is refused here
    /// while an enum, a literal union and `bool` are not.
    ///
    /// The primary span is the parameter rather than the attribute: the
    /// attribute is written correctly and it is the declared type that cannot
    /// answer it.
    pub const E_OPTION_TYPE_HAS_NO_CONVERSION: Code = Code::new("E0746");

    /// A `#[Route]` gives no `path`, no `method`, or neither.
    ///
    /// `rule:routing/route-attribute` marks only `name` optional, and a row of § 5's table is a
    /// path and a verb together: an attribute naming neither declares nothing
    /// and would be a route the author believes exists. Reported by the pass
    /// that builds the table rather than by the payload's roster check, for
    /// the reason `nvs_types::routes` states — a roster says what a field may
    /// hold, and *required* is a fact about the row.
    ///
    /// Both missing fields are named in one diagnostic: an author who wrote
    /// neither wrote one empty attribute, not two mistakes.
    pub const E_ROUTE_INCOMPLETE: Code = Code::new("E0747");

    /// Two `#[Route]`s declare the same verb and the same path shape.
    ///
    /// `rule:routing/routes-are-compiled-not-registered`'s duplicate-route error. § 2 matches by *shape*, so
    /// `/users/{id}` and `/users/{userId}` are one route however they are
    /// spelled, and the same path under a different verb is not a duplicate at
    /// all. A question about the whole enumeration rather than about one
    /// declaration, so it is reported once every file has been walked, at the
    /// declaration that arrives second in load order.
    pub const E_DUPLICATE_ROUTE: Code = Code::new("E0748");

    /// Two `#[Route]`s claim the same `name`.
    ///
    /// `rule:security/route-capture-is-laundered-by-its-type`'s second table-wide error, and the one § 4 rests on:
    /// `Core\Router::url` reverses the table by name, so a name that means two
    /// routes is a link with no answer. Reported at the second declaration,
    /// like [`E_DUPLICATE_ROUTE`], and at the `name:` field rather than at the
    /// whole attribute, because the rest of the attribute is fine.
    pub const E_DUPLICATE_ROUTE_NAME: Code = Code::new("E0749");

    /// A `#[Route]`'s `path` is not one `rule:routing/path-grammar`'s grammar admits.
    ///
    /// One code for the whole grammar, because every way a path fails it is
    /// the same fact — the router cannot build a node out of this — and the
    /// message names which way it was. The four: a path that does not begin
    /// at the root, a segment that is neither a literal nor a whole capture
    /// (`/u{id}`, `/{id}.json`), a capture naming nothing a parameter could
    /// be called, and a `{name?}` or `{name...}` written anywhere but last.
    ///
    /// § 2's "at most once" and "never in one path together" need no rule of
    /// their own: one segment is last, so a second trailing form is already
    /// in a position it is refused at.
    pub const E_ROUTE_PATH_GRAMMAR: Code = Code::new("E0750");

    /// A `{name}` capture names no parameter of the method it is attached to.
    ///
    /// `rule:security/route-capture-is-laundered-by-its-type`'s first compile error. A capture's value arrives *as* the
    /// parameter it is named after, so a capture with no parameter is a value
    /// with nowhere to go; the comparison is exact, per
    /// `rule:core-api/identifier-casing`, so
    /// `{userId}` and `$userid` are two names. The reverse is not an error: a
    /// parameter the path does not name is simply not the router's.
    pub const E_ROUTE_CAPTURE_UNBOUND: Code = Code::new("E0751");

    /// A capture's parameter is declared at a type no path segment converts
    /// to.
    ///
    /// `rule:security/route-capture-is-laundered-by-its-type`'s second compile error, over the roster
    /// `nvs_types::commands::converts_from_string` holds for both this and
    /// `rule:tooling/commands-are-compiled`'s `#[Option]` — one list, read twice. A `{name...}` is
    /// the one capture that roster does not answer for: § 3 hands a catch-all
    /// over as the single `tainted string` it was read as, nothing about it
    /// having been checked, so it binds a `string` and no other type.
    ///
    /// The primary span is the parameter, as with
    /// [`E_OPTION_TYPE_HAS_NO_CONVERSION`]: the path is written correctly and
    /// it is the declared type that cannot answer it.
    pub const E_ROUTE_CAPTURE_TYPE_HAS_NO_CONVERSION: Code = Code::new("E0752");

    /// A `{name?}` capture is bound to a parameter with no default.
    ///
    /// `rule:routing/path-grammar`: an optional capture matches one whole segment or none,
    /// and the default is what makes the absent case *well-typed* rather than
    /// nullable by accident — the router never invents a `null` for a
    /// parameter whose type does not admit one. Both sites are named, the
    /// parameter first, because the fix is a default rather than a rewritten
    /// path.
    pub const E_OPTIONAL_CAPTURE_NEEDS_DEFAULT: Code = Code::new("E0753");

    /// `Core\Router::url`/`urlAbsolute` names a route the table does not hold.
    ///
    /// `rule:routing/link-name-and-params-are-checked`: a *literal* `$name` that is not a declared route is a
    /// compile error, and a computed one throws instead — so this is reported
    /// at the argument rather than at the call, which is where the literal is.
    /// The table it is checked against is the whole program's, which is why
    /// the check runs after every file has been walked and not where the call
    /// is typed: the route may be declared in a file § 5's scan reaches later.
    pub const E_UNKNOWN_ROUTE_NAME: Code = Code::new("E0754");

    /// A `Core\Router::url` `$params` literal covers none of some capture the
    /// named route's path declares.
    ///
    /// `rule:routing/link-name-and-params-are-checked`: a `$params` array that does not cover the route's
    /// captures is a compile error. Only a `{name?}` may be absent — its
    /// segment simply is not emitted — so every `{name}` and `{name...}` the
    /// path writes needs a key of that name. A `$params` that is not an array
    /// literal is not checked at all: there are no keys to read, and § 4 asks
    /// nothing of a computed one.
    pub const E_ROUTE_LINK_MISSING_PARAM: Code = Code::new("E0755");

    /// A derived field's declared type is not in its format's type map.
    ///
    /// `rule:core-classes/derive-field-list`'s codec-reachable set for `#[Json\Derive]` and `rule:core-classes/db-column-types`'s type map for `#[Db\Derive]`, refused at the *declaration* that
    /// wrote it rather than at the `decodeAs<T>` or `queryAs<T>` that later
    /// runs. The two maps disagree — `bytes` is a `BLOB` column and has no
    /// JSON spelling, a nested class is a JSON object and no column at all —
    /// so the message names which one answered. It is the genuinely
    /// unmapped types this names and not the reachable ones `nvs_stdlib::json`
    /// still owes a decoder; the two are told apart in `nvs_types::derive`,
    /// which owns that split.
    pub const E_DERIVE_FIELD_NOT_CODEC_REACHABLE: Code = Code::new("E0756");

    /// A class carrying a derive attribute hand-writes every codec half that
    /// attribute would generate.
    ///
    /// `rule:core-classes/derive-generates-what-is-missing`: the derive generates only what the class does not
    /// declare itself, so a class writing both `toJson` and `fromJson` gets
    /// nothing from `#[Json\Derive]` — and an attribute with no effect is a
    /// mistake rather than a no-op. `Core\Db\Codec` declares `fromRow` alone,
    /// so for `#[Db\Derive]` one written member reaches the same conclusion.
    /// Reported at the attribute, which is the thing to delete.
    pub const E_DERIVE_BOTH_HALVES: Code = Code::new("E0757");

    /// A class carrying a derive attribute contributes no field to the codec.
    ///
    /// `rule:core-classes/derive-field-list`'s field list is the declared property list, so a class
    /// that declares no instance property — or skips every one it declares —
    /// derives an empty wire contract. § 7's rule about an attribute with no
    /// effect applies unchanged, and the fix is either a property or no
    /// attribute.
    pub const E_DERIVE_NO_FIELDS: Code = Code::new("E0758");

    /// A `Core\Router::url` `$params` key names neither a capture of the route's
    /// path nor one of its declared `#[Query]` parameters.
    ///
    /// `rule:routing/a-leftover-link-key-is-a-query-string`: keys that are not captures become the link's query string,
    /// so a key that covers nothing is not inert — it silently ships as
    /// `?typo=…`. The rule is what makes the query half safe to have at all,
    /// and it is the mirror of [`E_ROUTE_LINK_MISSING_PARAM`]: that one is a
    /// capture with no key, this one a key with nothing to be. Only the first
    /// of the two is reported for one call, because a misspelled key is
    /// usually both.
    pub const E_ROUTE_LINK_UNKNOWN_PARAM: Code = Code::new("E0759");

    /// An `#[Access]` gives no `allow`.
    ///
    /// `rule:attributes/access-payload` marks only `csrf` optional: the attribute exists to carry
    /// a decision, so one carrying none is exactly the omission § 3 refuses,
    /// written out instead of left out. Reported by the payload walk rather
    /// than by the roster check, for [`E_ROUTE_INCOMPLETE`]'s reason — a
    /// roster says what a field may hold, and *required* is not a fact a
    /// roster can state.
    pub const E_ACCESS_INCOMPLETE: Code = Code::new("E0760");

    /// An `#[Access]`'s `allow` value is not an enum case or a class constant.
    ///
    /// `rule:attributes/access-payload`: the field's declared type is `mixed` because § 2 is a
    /// promise *not* to know what the decision means, so a narrower type would
    /// be a claim the compiler does not make. What stands in place of the type
    /// is that the value **names** something — a bare literal names nothing,
    /// so `allow: "admin"` is the magic string the attribute exists to
    /// replace, and § 2's whole guarantee is that the name resolves.
    pub const E_ACCESS_ALLOW_NOT_A_NAME: Code = Code::new("E0761");

    /// A `#[Route]` method carries no `#[Access]`.
    ///
    /// ADR 0096 §§ 1 and 3, and the whole of why that ADR exists: there is no
    /// implicit `Public` and no configuration that supplies one, so a route
    /// that is open because its author decided so and one that is open because
    /// its author forgot are not the same text. Reported at the `#[Route]`
    /// rather than at the method, because the attribute is what makes the
    /// declaration owe a decision — a method with neither attribute owes
    /// nothing.
    ///
    /// The mirror of [`E_ACCESS_INCOMPLETE`]: that one is a decision that
    /// declares nothing, this one a route that declares no decision.
    pub const E_ROUTE_WITHOUT_ACCESS: Code = Code::new("E0762");

    /// One method carries two `#[Access]` attributes.
    ///
    /// `rule:attributes/access-payload`'s last rule. `rule:attributes/repeatable` makes every attribute repeatable and leaves the ambiguity to
    /// retrieval, which is exactly what cannot happen here: two decisions are
    /// two readings — every one of them, or any one of them — and choosing
    /// between them silently is the failure `rule:security/access-is-checked-for-presence-not-meaning` exists to prevent.
    /// Reported at the second, naming the first, because the first is the one
    /// an author reading the error is deciding whether to keep.
    pub const E_ACCESS_REPEATED: Code = Code::new("E0763");

    /// A `csrf: false` beside a method whose every `#[Route]` names a safe verb.
    ///
    /// `rule:security/csrf-is-on-by-default`'s opt-out, held to the thing it opts out of: CSRF is on by
    /// default for `Post`, `Put`, `Patch` and `Delete` and for no other verb, so
    /// beside a method declaring none of those the field turns nothing off.
    /// Refused rather than ignored because a field that reads as a security
    /// decision and has no effect is exactly how one ends up pasted onto routes
    /// that never needed it — and a reader auditing those routes then has to
    /// re-derive which of them the runtime was ever going to check.
    pub const E_CSRF_WITHOUT_UNSAFE_VERB: Code = Code::new("E0764");

    /// A `#[Query]` on a parameter of a method carrying no `#[Route]`.
    ///
    /// `rule:routing/a-query-parameter-is-declared-like-a-capture` gives the marker its whole meaning on a route method's
    /// parameter — the key it binds by is the parameter's own name — so one
    /// written anywhere else binds nothing and is read by nothing. Refused
    /// rather than ignored for the reason the closed roster of recognized
    /// attributes exists at all: a name the compiler knows, sitting where the
    /// compiler never looks, reads to its author as a declaration that works.
    ///
    /// The sibling of [`E_OPTION_WITHOUT_COMMAND`], which is the same mistake
    /// made with the other pass's marker.
    pub const E_QUERY_WITHOUT_ROUTE: Code = Code::new("E0765");

    /// An `#[Option]` on a parameter of a method carrying no `#[Command]`.
    ///
    /// `rule:tooling/commands-are-compiled`'s marker, held to the declaration that reads it, exactly as
    /// [`E_QUERY_WITHOUT_ROUTE`] holds `rule:routing/a-query-parameter-is-declared-like-a-capture`'s. Reported from the walk
    /// over every method rather than from the command pass, which by
    /// construction sees only the methods a `#[Command]` selects.
    pub const E_OPTION_WITHOUT_COMMAND: Code = Code::new("E0766");

    /// A `#[Command]` that names no command.
    ///
    /// `rule:tooling/commands-are-compiled` selects a command by the `name` a command line writes, so
    /// a row without one is reachable by nothing and the table pass is what
    /// discovers it — the same reading [`E_ROUTE_INCOMPLETE`] gives a
    /// `#[Route]` that gave no path, and the answer to that module's own
    /// question of whether `name` is required.
    pub const E_COMMAND_WITHOUT_NAME: Code = Code::new("E0767");

    /// Two `#[Command]`s claiming one name — the first of `rule:tooling/commands-are-compiled`'s
    /// compile errors, and the one only the whole program's enumeration can
    /// answer.
    ///
    /// [`E_DUPLICATE_ROUTE_NAME`]'s sibling, reported over the collected table
    /// for its reason: a command line naming a word two methods answer to has
    /// no answer, and which of them ran would depend on the order the files
    /// were walked in.
    pub const E_DUPLICATE_COMMAND: Code = Code::new("E0768");

    /// A literal argument to an `rule:expressions/intrinsic-list-is-closed` intrinsic that its own grammar
    /// refuses.
    ///
    /// The whole point of that ADR, as one code: the compiler read the
    /// constant with the parser the runtime would have used, so this is the
    /// throw the first request to reach the call would have taken, moved to
    /// `nvs check`. One code across the grammars rather than one per member —
    /// the message carries the parser's own words, and a reader searching for
    /// "malformed pattern" should not have to know which member made the
    /// refusal.
    pub const E_INTRINSIC_LITERAL_MALFORMED: Code = Code::new("E0769");

    /// A literal `Core\Str::format` template, or a literal `Core\Db` query,
    /// that does not fit the arguments written beside it.
    ///
    /// Separate from [`E_INTRINSIC_LITERAL_MALFORMED`] because the literal is
    /// *fine*: the mistake is in the pairing, which is `rule:expressions/intrinsic-list-is-closed`'s own
    /// reason for naming this member's placeholder check separately — it turns
    /// PHP's `printf` argument-mismatch bug family into a compile error. ADR
    /// 0067 § 10's placeholder count and its positional-vs-named consistency
    /// are the same question of a second grammar, so they are the same code:
    /// the message carries the reader's own words, per
    /// [`E_INTRINSIC_LITERAL_MALFORMED`]'s.
    pub const E_FORMAT_TEMPLATE_MISMATCH: Code = Code::new("E0770");

    /// An `#[Api]` that contradicts the code it annotates — `rule:attributes/api-adds-and-cannot-contradict`'s
    /// contradictions, under one code.
    ///
    /// One code rather than several because § 2 states them as one rule: the
    /// annotation *may add, and may not contradict*. Each of them is a
    /// different way for the same sentence to be false, so what distinguishes
    /// them is the message and the two spans it names, not a number an author
    /// would ever look up separately. The fix is the same in every case —
    /// correct the attribute, or correct the code it disagrees with.
    pub const E_API_CONTRADICTS_THE_CODE: Code = Code::new("E0771");

    /// A `Core\Router::url` `$params` value is a literal outside the closed set
    /// the parameter it supplies declares.
    ///
    /// `rule:routing/a-capture-narrows-to-a-closed-set`: a capture narrows to a closed set with a *type*, and a
    /// path segment outside that set fails the conversion and falls through to
    /// `404`. A link built out of such a value is therefore a link to a route
    /// that will not match — the one shape of dead link the table can see
    /// before the program runs, and the mirror of
    /// [`E_ROUTE_LINK_UNKNOWN_PARAM`]: that one is a key that names nothing,
    /// this one a key that names something and cannot hold what it was given.
    /// Only a *literal* value is checked, exactly as § 6's key rule checks only
    /// a literal key — a computed value has nothing to read.
    pub const E_ROUTE_LINK_VALUE_NOT_IN_SET: Code = Code::new("E0772");

    /// A `secret`-qualified value reaches
    /// `rule:classes/graph-copy`'s graph copy — [ADR
    /// 0033](/docs/decisions/0033.md)
    /// § 4's `serialize()`-and-`spawn` sink, which that bullet deliberately
    /// states once for *both* carriers rather than distinguishing "crossing to
    /// a live isolate" from "externalizing to bytes".
    ///
    /// It is one code for both because the disclosure is one disclosure: the
    /// walk that puts a credential into bytes on disk is the walk that puts it
    /// into another arena, and a developer who learns the rule at one carrier
    /// has learned it at the other. The call sites that report it are
    /// `Core\Serialize::encode`, `spawn script`'s `args:`,
    /// `rule:concurrency/an-upgrade-is-spawn-shaped`
    /// 's `Core\Socket::upgrade` — an `args:` that opens a connection
    /// isolate rather than a script one — and § 4's `Core\Topic::publish`,
    /// whose value is copied into every subscriber's arena on every core.
    /// `spawn`/`spawn worker` join them from the same check once they
    /// compile.
    ///
    /// A call-site rule rather than a parameter type, for the same reason
    /// [`E_SECRET_DEBUG_ARGUMENT`] is one: `encode` declares `mixed`, which a
    /// `secret string` satisfies, so the argument expression is the last place
    /// the qualifier is visible. A separate code from that one because the
    /// destination is not a channel a person reads — nothing is disclosed
    /// until the bytes are, which is exactly why the refusal has to be at the
    /// copy rather than at some later read nobody can see from here.
    ///
    /// The qualifier one storage kind along is *not* this code: a `secret`
    /// **property** of an object being copied is refused by the walk itself at
    /// run time (`nvs_runtime::graph`), because the object's static type is
    /// what a call site sees and its properties are not.
    pub const E_SECRET_CROSSES_A_BOUNDARY: Code = Code::new("E0775");
    // `E0776` is retired and is never reused: `await` lowers, as `spawn
    // script` does at `E0703`.
    // `E0777` is retired and is never reused: `spawn script`'s `limits:`,
    // `grants:` and `on:` are checked where they are written
    // (`nvs_types::expr::isolate`), and an option the compiler checks has no
    // refusal left to carry. A placement that is neither word is
    // `E_SPAWN_PLACEMENT_UNKNOWN` and a `limits:` key that is no sub-cap is
    // `E_UNKNOWN_OPTION`, each reported at the option it is about.

    /// A user-declared class's non-static method reached through the class
    /// name from a frame that holds no `$this` — `C::f()` at file scope, or
    /// inside a `static` method of any class.
    ///
    /// [`E_CORE_INSTANCE_MEMBER_CALLED_STATICALLY`] is the same mistake
    /// against a `Core` class, kept separate because its wording is `rule:core-api/shape-rules`
    /// R20's one-spelling rule rather than this one's missing receiver. This
    /// code is the sibling `rule:statements/static-is-a-member-modifier` asks for: `static` keeps PHP's
    /// semantics unchanged, so a method without it is called on a value.
    ///
    /// `self::f()` and `parent::f()` from an *instance* method are not this —
    /// they forward the frame's own `$this`, which is why the question asked
    /// is whether one is in scope rather than how the call is spelled.
    pub const E_INSTANCE_METHOD_CALLED_STATICALLY: Code = Code::new("E0778");

    /// `$this` written where no receiver is in scope — a `static` method's
    /// body, a plain function's, or the file scope.
    ///
    /// Separate from [`E_UNDEFINED_VARIABLE`] because `$this` is never
    /// *declared* by anything a program writes: the fix is not an assignment
    /// but a different method, so a message about a missing declaration would
    /// send the reader looking for one to add.
    pub const E_THIS_WITHOUT_A_RECEIVER: Code = Code::new("E0779");

    /// `throw` of a value that is not a `Throwable` — a scalar, an `array<T>`,
    /// an enum, or a class outside spec § 10's tree.
    ///
    /// The tree's root is what `catch` matches against and what
    /// `nvs_ir::lower::exception` builds a landing pad for, so an operand
    /// outside it has nothing to be caught by; the lowerer refuses to lower
    /// one at all.
    pub const E_THROW_OPERAND_NOT_THROWABLE: Code = Code::new("E0780");

    /// `clone` of a value that can hold no object — `rule:classes/clone-is-shallow` makes it "a
    /// new instance of `$x`'s class", and a scalar, an `array<T>` or an enum
    /// names no class to instantiate.
    ///
    /// An `array<T>` is the operand worth naming in the help rather than
    /// merely refusing: it is already copied by assignment (`rule:programs/memory-priority`'s
    /// copy-on-write), so the `clone` a reader reaches for is not missing but
    /// unnecessary.
    pub const E_CLONE_OPERAND_NOT_AN_OBJECT: Code = Code::new("E0781");

    /// A write to a `readonly` property from anywhere but the declaring
    /// class's own `constructor` — `rule:classes/lateinit-restrictions`'s contract for the modifier,
    /// "assigned exactly once, and that assignment happens during
    /// construction".
    ///
    /// Refused where the write is written rather than left to a run-time
    /// check, which is what makes the modifier mean anything at all: PHP
    /// admits an initializing write from any method of the declaring class and
    /// throws only on the second one, so the property is write-once by
    /// bookkeeping there and by the type system here.
    pub const E_READONLY_WRITE_AFTER_CONSTRUCTION: Code = Code::new("E0782");

    /// A class names a `final` class as its superclass. PHP refuses the same
    /// declaration, and for the same reason: `final` is the author's statement
    /// that the class's behaviour is not extended, so an `extends` naming one
    /// is a contradiction rather than a widening.
    pub const E_FINAL_CLASS_EXTENDED: Code = Code::new("E0783");

    /// A class redeclares a method an ancestor declared `final`.
    ///
    /// Separate from [`E_FINAL_CLASS_EXTENDED`] because the two are separate
    /// promises — a class that may be extended can still hold a member that
    /// may not be replaced — and a reader fixing one is not fixing the other.
    pub const E_FINAL_METHOD_OVERRIDDEN: Code = Code::new("E0784");

    /// `new` names a declaration that has no instances — an `abstract` class
    /// or an interface. Both leave members without a body, so the object it
    /// would allocate could not answer every call its own type admits.
    pub const E_ABSTRACT_INSTANTIATED: Code = Code::new("E0785");

    /// A class that is not `abstract` declares a method with no body.
    ///
    /// Separate from [`E_ABSTRACT_INSTANTIATED`] because it is the other
    /// direction of the same rule — that one refuses the instance, this one
    /// refuses the hole — and a class whose declaration is fixed here has no
    /// `new` to fix.
    pub const E_ABSTRACT_METHOD_IN_CONCRETE_CLASS: Code = Code::new("E0786");

    /// A write to a property that declares a `get` hook and no `set` hook.
    /// The accessor pair is the whole of what such a property answers with,
    /// so a write has nothing to commit through.
    pub const E_GET_ONLY_HOOK_WRITE: Code = Code::new("E0787");

    /// An `#[Access]` on a method carrying no `#[Route]`.
    ///
    /// `rule:attributes/access-is-a-required-sibling` makes the attribute a *sibling* of `#[Route]`, and
    /// [`E_ROUTE_WITHOUT_ACCESS`] is that sentence read in the other
    /// direction: a route must declare a decision. This one refuses the
    /// decision that guards no route — § 2 promises the compiler will not
    /// interpret the `allow` value, so away from the route table there is
    /// nothing that ever reads it. The sibling of [`E_QUERY_WITHOUT_ROUTE`]
    /// and [`E_API_CONTRADICTS_THE_CODE`], which are the same mistake made
    /// with the other markers a `#[Route]` gives meaning to.
    pub const E_ACCESS_WITHOUT_ROUTE: Code = Code::new("E0788");

    /// A `#[Command]` method that is not `static`, or that returns something
    /// other than `void` or `uint`.
    ///
    /// `rule:tooling/commands-are-compiled`'s two facts about the declaration the attribute sits on,
    /// under one code for [`E_TEST_METHOD_SHAPE`]'s reason: they are one
    /// question — whether this declaration is a command handler — and an
    /// author fixing either is editing the same line.
    pub const E_COMMAND_METHOD_SHAPE: Code = Code::new("E0789");

    /// A `secret`-qualified value written by `echo` or `print`.
    ///
    /// `rule:security/secret-sinks-refuse`'s terminal-output sink, at the two statements that reach
    /// it without a member in between. It is refused **with no carrier
    /// bypass** — the value is being displayed to a person rather than used,
    /// and neutralizing a control byte does nothing for confidentiality —
    /// and the name understates the reach: `rule:tooling/echo-always-has-a-sink` routes a scheduled
    /// script, a job worker, a `#[Test]` method and a `spawn script` isolate
    /// through this same sink, so the destination is as often a CI log or a
    /// test report as a terminal.
    ///
    /// A separate code from [`E_SECRET_DEBUG_ARGUMENT`] because the way in is
    /// different: a dump is an argument to a member that declares `mixed`,
    /// while this one is an operand of a statement, and the operand is
    /// commonly an interpolation the qualifier spread to rather than the
    /// secret binding itself. The help says so, because that is the half an
    /// author does not expect.
    pub const E_SECRET_OUTPUT: Code = Code::new("E0790");

    /// A `secret`-qualified value reaching `Core\Json::encode`.
    ///
    /// `rule:security/secret-sinks-refuse`'s serialiser bullet: an encoded document is on its way to
    /// a response, a log or a queue, and none of those is the credential
    /// being *used*. Refused at the call site for
    /// [`E_SECRET_DEBUG_ARGUMENT`]'s reason — `encode` declares `mixed`, so
    /// the written argument is the last place the qualifier is visible.
    ///
    /// A separate code from [`E_SECRET_CROSSES_A_BOUNDARY`], which is the
    /// same shape at a different destination: that one refuses a graph copy
    /// on its way to an isolate or to `serialize()`'s bytes, where the value
    /// is still the program's own. This one refuses a document written for
    /// something outside it, and the way out differs to match — `reveal` at
    /// the one field that must travel, rather than at the call.
    pub const E_SECRET_ENCODED: Code = Code::new("E0791");

    /// A read of a class constant whose declared value has no compile-time
    /// form.
    ///
    /// `rule:classes/no-free-functions-or-constants` inlines a class constant at every use site — there is no
    /// storage a read could load it from — so a declaration the constant
    /// folder cannot reduce to a value has nothing to lower. Reported at the
    /// **read**, not at the declaration, because the declaration alone is
    /// harmless: a constant nobody names costs nothing, and the span the
    /// author can act on is the one that asked for a value.
    ///
    /// Distinct from [`E_LITERAL_TYPE_NOT_CONST`], which refuses the same
    /// declaration in *type* position under `rule:types/constant-in-type-position`'s narrower question
    /// ("is this a `string` or `int` literal type"). An `array` constant is
    /// legal here and refused there, and after the array fold this code is
    /// down to the shapes no constant emitter exists for at all — another
    /// class's `const`, an enum case, and `Foo::class` nested inside a
    /// container.
    pub const E_CLASS_CONST_NO_CONSTANT_FORM: Code = Code::new("E0792");

    /// `rule:types/callable-is-a-closure`'s `(...)` naming a member whose parameter list a
    /// `callable` cannot carry — one declared `inout $x`, or a variadic tail.
    ///
    /// `rule:types/callable-absorbs-closure` gives `callable` no parameter list, so a call *through* one passes
    /// what it was written with and nothing else: there is no site that could
    /// know to stage a by-reference cell, and none that could know to collect
    /// a tail into the one array the callee reads that slot as. Both are
    /// therefore a type confusion in the callee rather than a wrong answer,
    /// which is why this is a refusal and not a run-time report.
    ///
    /// The `inout` half is [`E_CLOSURE_INOUT_PARAM`]'s rule reached by the
    /// other spelling — that code refuses the parameter where a closure
    /// *declares* it, this one refuses naming a member that already has one.
    pub const E_FIRST_CLASS_CALLABLE_UNFORWARDABLE: Code = Code::new("E0793");

    /// `rule:classes/constructor-compatibility`: a `new` through a `class<T>` whose `T` has an implementor
    /// declaring a constructor incompatible with `T`'s.
    ///
    /// The site cannot see which implementor the value holds, so `T`'s own
    /// constructor is the only signature it can check against — and that check
    /// is only sound if every implementor accepts what `T` accepts. Refused at
    /// the `new` rather than at the divergent class's declaration, because a
    /// subclass never instantiated through a class reference is nobody's
    /// problem and refusing it at the declaration would make an unrelated
    /// file's `new` the reason a class cannot be written.
    pub const E_DYNAMIC_NEW_DIVERGENT_CONSTRUCTOR: Code = Code::new("E0794");

    /// `rule:types/class-reference`: a `class<T>` whose argument is not a class or an
    /// interface — `class<int>`, `class<array<Dog>>`, an enum name.
    ///
    /// A class reference's value is a class descriptor, so the argument is the
    /// hierarchy bound every descriptor it can hold conforms to, and nothing
    /// but a class or an interface names one. Distinct from
    /// [`E_UNDEFINED_CLASS`], which is the *other* mistake `class<...>` admits:
    /// a name that resolves to nothing at all. `crate::lower`'s
    /// `lower_class_ref` reports this one only when the argument lowered
    /// without complaint of its own.
    pub const E_CLASS_REF_ARGUMENT_NOT_A_CLASS: Code = Code::new("E0795");

    /// `rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key`: a request member whose verb repeats an effect —
    /// `Core\Http\Client::post` — asking for retries without
    /// `retryIdempotencyKey`.
    ///
    /// Reportable while compiling because both halves are written: the verb is
    /// the member's own name, and `rule:core-api/shape-rules` R2 makes the options bag a literal
    /// at the call site. Distinct from [`E_UNKNOWN_OPTION`], which is the
    /// mistake of naming an option that does not exist; here every option
    /// named is real and it is the *absent* one that is the defect.
    pub const E_RETRY_WITHOUT_IDEMPOTENCY_KEY: Code = Code::new("E0796");

    /// A `secret`-qualified value reaching `Core\Log::write`.
    ///
    /// `rule:security/secret-sinks-refuse`'s log bullet, which is the *opposite* default from
    /// `tainted`: `rule:security/sink-predicate` explicitly wants untrusted input logged, and a
    /// credential is the one thing a record must not carry. Refused at the
    /// call site for [`E_SECRET_ENCODED`]'s reason and one more of its own —
    /// `fields` is declared `array<mixed>` **by design**, so the parameter
    /// type is deliberately not the thing that refuses, and the written
    /// argument is the last place the qualifier is visible.
    ///
    /// A separate code from [`E_SECRET_ENCODED`], which refuses a document on
    /// its way *somewhere*: a log record has already arrived, and the way out
    /// differs to match — the field a program genuinely means to record is
    /// revealed by name, which is what makes a logged credential a written
    /// decision rather than an accident of what was in the bag.
    pub const E_SECRET_LOGGED: Code = Code::new("E0797");

    /// A `Core\Attributes` retrieval whose literal `$member` names no declared
    /// parameter or property of the target.
    ///
    /// `rule:attributes/structural-retrieval`'s last paragraph: a written `$member` is checked against
    /// the target's real declarations at the call site, the same
    /// literal-inspection `rule:security/secret-sinks-refuse`'s sinks make. Only a *computed*
    /// `$member` falls back to the empty result § 4's *Consequences* fixes,
    /// because there is no literal left to check.
    ///
    /// Distinct from [`E_ATTRIBUTE_TARGET_NOT_A_DECLARATION`], which is the
    /// mistake of writing something that is not a declaration reference at
    /// all: here the target resolves and it is the member name inside it that
    /// reaches nothing. Reported instead of folding, since the fold a
    /// misspelling produces — `null`, or the empty array — is exactly what a
    /// correct retrieval of an absent attribute produces, and nothing later
    /// can tell the two apart.
    pub const E_ATTRIBUTE_MEMBER_NOT_DECLARED: Code = Code::new("E0798");

    /// `rule:types/property-key`: a `property<T>` whose argument is not a class —
    /// `property<int>`, an interface, an enum.
    ///
    /// A property key's values are the names of `T`'s public declared
    /// properties, so the argument has to be something that declares
    /// properties. An interface is refused with the rest and the ADR says why
    /// it is the conservative row: its implementors satisfy it with storage it
    /// does not itself declare, so the set would name nothing the receiver
    /// certainly has.
    ///
    /// [`E_CLASS_REF_ARGUMENT_NOT_A_CLASS`]'s sibling, one band apart from it
    /// only because this band had one number left, and reported under the same
    /// guard: `crate::lower`'s `lower_property_key` speaks only when the
    /// argument lowered without a complaint of its own, so an unresolvable name
    /// stays [`E_UNDEFINED_CLASS`] alone.
    ///
    /// **The name is the first of its two rows, not the whole code.** `rule:types/property-key` refuses an argument that is not a class *and* a class whose public
    /// property set is empty, "since no value of that type could ever exist" —
    /// one rule about what `T` may be, so one code. The second row is reported
    /// from `nvs_types::expr::operators::reject_empty_property_key_set`, at the
    /// conversion rather than at the lowering, and that function's doc owns why
    /// the two halves cannot share a site.
    ///
    /// **The last code in the E07xx band, which is full.** The types band has
    /// filled twice — at `E0499` and here — and the next types diagnostic
    /// opens a new band rather than taking `E0800`, whose digits read as no
    /// band at all. That is a project-level decision and `docs/adr/README.md`
    /// § *Decisions taken at project level* is its home, as it is for `E0500`.
    pub const E_PROPERTY_KEY_ARGUMENT_NOT_A_CLASS: Code = Code::new("E0799");

    // --- E08xx types, continued again ---------------------------------------
    //
    // The third band for one stage. `E07xx` filled at `E0799`, whose own doc
    // comment says the next types diagnostic opens a band rather than taking
    // the number past the end of that one, and `docs/adr/README.md`
    // § *Decisions taken at project start* sets `E08xx` aside for whichever
    // band fills next — it is the project-level home of both.
    //

    /// `callable(int $x): string` — a parameter name inside a `callable` type,
    /// `rule:types/callable-signature`.
    ///
    /// The band's first code, claimed by ADR 0136 § *Diagnostics* before
    /// anything below it was written, which is why it sits ahead of the
    /// numbers rather than filling a hole. A name in the type would imply
    /// calling through the value by name, which nothing supports; reported
    /// once per type however many parameters carry one, since writing PHP's
    /// parameter spelling is one mistake and deleting the names is one fix.
    pub const E_CALLABLE_TYPE_NAMES_A_PARAMETER: Code = Code::new("E0800");

    /// Two writers of one response body — `echo` and a typed body member, or
    /// two different typed members —
    /// `rule:security/response-body-is-one-typed-member`
    /// 's sixth row.
    ///
    /// They disagree about the body's type and its `Content-Type`, and
    /// letting the last one win is how a JSON endpoint acquires an HTML
    /// prelude. One member called twice is one writer and is not this
    /// diagnostic. Reported inside a `#[Route]` handler and nowhere else, because
    /// § 3 binds `echo` by context and a handler is the one body a request is
    /// statically certain to reach — `nvs_types::response`'s module doc owns
    /// that scope, the reach inside a handler, and the entry-script gap it
    /// leaves.
    pub const E_ECHO_BESIDE_A_BODY_MEMBER: Code = Code::new("E0801");

    /// A `spawn script` operand that is neither of
    /// [ADR 0006](/docs/decisions/0006.md)
    /// § *Decision*'s two entry forms — an `fn` literal, or a value whose type
    /// is `callable`.
    ///
    /// One code for both because it is one rule: the entry is **decided
    /// syntactically at the spawn site**, a path or a `Class::method(...)`
    /// reference written there, and everything else is refused for the same
    /// reason — whether a callable captures is not a question the spawn site
    /// can answer, and an isolate that captured would share more than compiled
    /// code. The two halves differ only in the `help:`, because an `fn` literal
    /// has a mechanical way out (give it a name and a class) and a variable
    /// does not.
    ///
    /// Deliberately not [`E_TYPE_MISMATCH`], which is what a `callable` against
    /// a `string` parameter would read as: the operand position accepts two
    /// unrelated shapes, so "expected `string`" describes half the rule and
    /// sends the reader to fix the wrong half.
    pub const E_SPAWN_ENTRY_NOT_A_PATH_OR_METHOD: Code = Code::new("E0802");

    // `E0804` is retired and is never reused: it refused a method entry that
    // declared a parameter. ADR 0006 § *Decision*'s binding lowers — the
    // entry's parameter names ride on the spawn symbol and
    // `nvs_stdlib::script` binds `args:`'s entries to them by name — so that
    // shape is the shape that runs, and a mismatch is the ordinary
    // named-argument error the ADR names, raised at the spawn.

    /// A written reason that is not a source literal, at a member whose reason
    /// exists to be read by the next person —
    /// `rule:core-classes/html-to-source`'s second sentence, which `nvs_types::reasons` owns the roster of.
    ///
    /// **Only the half a compiler can answer.** The same sentence also refuses
    /// an *empty* reason, and that stays the member's own throw: emptiness is a
    /// property of the text, which the body reads whatever the site wrote, and
    /// a second refusal here would only make the throw unreachable.
    /// `nvs_types::reasons`' module doc is the home of the split.
    ///
    /// A `const` **is** a source literal here, deliberately: it folds, so the
    /// text is still in the source and still greppable, and pulling a long
    /// justification out to a named constant is the shape this hatch wants
    /// rather than the one it refuses.
    pub const E_REASON_NOT_A_SOURCE_LITERAL: Code = Code::new("E0805");

    /// A `Core\Db\…::queryAs<T>` whose `T` no row can be hydrated into —
    /// `rule:core-classes/db-column-types`'s type map, asked at the call
    /// rather than at a declaration.
    ///
    /// One code for the three ways a written `T` fails that question, because
    /// they are one rule read at one site: the type is a **list**
    /// (`array<Row>`, where § 4 already answers `Core\Db\Rows` of one), the
    /// class carries no `#[Db\Derive]` and so has no mapping at all, or it
    /// carries one whose mapped fields cannot fill its `constructor` — a
    /// property `#[Db\Field(skip: true)]` took off the mapping is the shape
    /// that reaches this, and `nvs_types::derive`'s own doc says why that
    /// refusal cannot be made where the class is declared.
    ///
    /// Deliberately not [`E_DERIVE_FIELD_NOT_CODEC_REACHABLE`], which is the
    /// declaration's own refusal and fires whether or not anything ever calls
    /// `queryAs`: this one is about a *call*, and the class it names may be
    /// perfectly well formed for every other purpose it has.
    pub const E_QUERY_AS_NOT_A_ROW_CLASS: Code = Code::new("E0806");

    /// `callable(int)` — a `callable` type written with a parameter list and
    /// no return type, `rule:types/callable-signature`.
    ///
    /// The return type is mandatory because `callable(int)` says strictly less
    /// than bare `callable` while costing a second spelling of "unknown": the
    /// arguments would be proven where the call is written and the result
    /// would not, so the site keeps the dynamic path anyway. `void` and
    /// `never` are writable there as in any other return position, which is
    /// what the help names.
    pub const E_CALLABLE_TYPE_WITHOUT_RETURN: Code = Code::new("E0807");

    /// `fn ($n) => …` written where nothing says what `$n` holds —
    /// `rule:types/callable-literal-inference`.
    ///
    /// A closure literal's parameter may leave its type out, and then takes it
    /// from the position the literal is written in. Only a written
    /// `callable(...)` signature is such a position: bare `callable` is the top
    /// of the lattice and names no parameter, and a signature shorter than the
    /// literal names this one no type either. The parameter is checked as
    /// `mixed` afterwards so the body is still checked at all.
    pub const E_CLOSURE_PARAMETER_TYPE_NOT_INFERABLE: Code = Code::new("E0808");

    /// `$f(1)` where `$f` is a `callable(int, string): R` — a call through a
    /// written signature passing a number of arguments the signature does not
    /// name, `rule:types/callable-signature`.
    ///
    /// The count is exact in this one direction only: the *value* may hold a
    /// closure of any arity up to the signature's
    /// (`rule:types/callable-arity`), and the runtime hands that closure only
    /// the leading arguments it declares — so a site passing fewer than the
    /// signature names can leave a parameter of the closure actually held
    /// unfilled, which is a fault below the language rather than a throw. A
    /// site passing more names a parameter the type does not have. Bare
    /// `callable` keeps no count at all: nothing there says what to compare
    /// against.
    pub const E_CALLABLE_CALL_ARITY: Code = Code::new("E0809");

    /// `Core\Request::jsonAs<T>()` over a `T` holding a `string` or `bytes`
    /// property that is not written `tainted` —
    /// `rule:security/derived-codec-qualifiers`.
    ///
    /// A derived codec assigns a peer's octets straight into the declared
    /// property types, so the qualifier a field declares has to be the one the
    /// payload carries. The codec cannot ask that question: it is derived from
    /// a class that says nothing about where its documents come from, and the
    /// same class is legitimate over one the program built itself. The call
    /// that decodes is where the qualifier is statically known, so that is
    /// where it is asked.
    ///
    /// The message names the **property**, not the call, because writing
    /// `tainted` on that property is the whole fix and the site has nothing to
    /// change. Only `string` and `bytes` carry a qualifier
    /// (`rule:security/tainted-qualifier`), so a `T` of integers, decimals,
    /// enums or instants never reaches this.
    ///
    /// It is `jsonAs`'s alone rather than every decoder's: `Core\Json::decodeAs`
    /// takes its document through a plain `string` parameter, so a tainted
    /// argument is already refused where it is passed.
    pub const E_DECODED_FIELD_NOT_TAINTED: Code = Code::new("E0810");

    /// `$x is void`, `$x is never` — a type no value inhabits, written where
    /// `is` asks whether a value holds one (`rule:types/type-test`, the second
    /// of its three refusals).
    ///
    /// Not a knowable answer this refuses but an unaskable question: `void` is
    /// what a function returns instead of a value and `never` is what one
    /// returns by not returning, so neither names a shape a subject could be
    /// carrying. `is` is total over every type a value can inhabit
    /// (ADR 0150 § 6), and these two are the boundary of that set rather than
    /// an exception inside it.
    pub const E_TYPE_TEST_AGAINST_AN_UNINHABITED_TYPE: Code = Code::new("E0811");

    /// `$x is $cls` — a value written where `is` takes a type
    /// (`rule:types/type-test`, the third of its three refusals).
    ///
    /// The spelling is not ours to give a meaning to: PHP's grammar binds a
    /// variable in that slot, so reading it as a dynamic class test would make
    /// one line mean two different things in the two languages, silently, in
    /// both. `$x instanceof $cls` is that test and is the one question `is`
    /// cannot ask, so the help names it.
    ///
    /// It is reported in the parser, where the `$` is still in hand. Deferred
    /// to name resolution it would arrive as a type that failed to resolve,
    /// and the diagnostic would describe the name rather than the shape.
    ///
    /// `rule:types/type-test`'s table is the home of which code refuses what.
    pub const E_TYPE_TEST_AGAINST_A_VALUE: Code = Code::new("E0812");

    /// `$x is tainted string`, `$x is secret bytes` — a qualifier written where
    /// `is` takes a type (`rule:types/type-test`, the first of its three
    /// refusals).
    ///
    /// `tainted` and `secret` are checked once and erased before codegen
    /// (`rule:security/tainted-qualifier`, `rule:security/secret-qualifier`):
    /// no byte of a value says where it has been, so there is nothing for a
    /// run-time test to read. The answer is not merely knowable here, the way a
    /// test the declaration settles is — it is absent, which is why this is a
    /// refusal and a statically-true test is not.
    ///
    /// Numbered past its two siblings because `E0810` was already
    /// [`E_DECODED_FIELD_NOT_TAINTED`] when `rule:types/type-test` was written,
    /// and a code means one thing.
    pub const E_TYPE_TEST_AGAINST_A_QUALIFIER: Code = Code::new("E0813");

    /// A `catch` clause or arm naming `Core\Script\Finished`, the class
    /// `Core\Script::finish()` raises.
    ///
    /// The marker is a root of its own rather than a `Throwable`
    /// (`nvs_hir::errors::TREE`'s own docs are the mechanism), so an arm naming
    /// it already matches nothing at run time: the refusal is what keeps that
    /// from reading as a compiler bug. A site writing one has understood the
    /// ending backwards — a finish is not a failure a request recovers from,
    /// and the code that must run on the way out belongs in a `finally`, which
    /// the unwind runs, or in a `Core\Script::onExit` hook, which the ending
    /// fires.
    ///
    /// Separate from [`E_THROW_OPERAND_NOT_THROWABLE`], which refuses the
    /// *other* end of the same mistake and needs no case of its own: the marker
    /// descends from `Throwable` not at all, so `throw` already reports it as
    /// an operand outside the tree.
    pub const E_CATCH_ARM_NAMES_THE_FINISH_MARKER: Code = Code::new("E0814");

    /// Two of `Core\Http\Client`'s four body keys written at one call.
    ///
    /// `rule:http-server/an-outbound-request-carries-one-body`: which key a body
    /// was written under is what says how it is sent, so two of them is not a
    /// request with two bodies but a call site that has not decided what it is
    /// sending. Reportable while compiling for
    /// [`E_RETRY_WITHOUT_IDEMPOTENCY_KEY`]'s reason — `rule:core-api/shape-rules` R2 makes the bag
    /// a literal — and where the verb is dynamic the same question throws
    /// before the first attempt rather than after it.
    pub const E_TWO_REQUEST_BODIES: Code = Code::new("E0815");

    /// A body key written at a `Core\Http\Client` member whose verb carries no
    /// body.
    ///
    /// `rule:http-server/an-outbound-request-carries-one-body` again, at its
    /// other half: a `GET` and a `HEAD` ask a question, and octets attached to
    /// one are read by no server the request was worth making to. Distinct from
    /// [`E_TWO_REQUEST_BODIES`], where the keys are legal for the verb and it is
    /// their number that is the defect.
    pub const E_BODY_ON_A_BODYLESS_VERB: Code = Code::new("E0816");

    /// `contentType` written at a `Core\Http\Client` member with no `body`.
    ///
    /// The key types [`E_TWO_REQUEST_BODIES`]'s raw octets and nothing else: a
    /// `json`, a `form` and a `multipart` body each carry the media type their
    /// key already named, so a `contentType` beside one is either a second
    /// opinion the request will not send or a `body` the call site forgot to
    /// write.
    pub const E_CONTENT_TYPE_WITHOUT_A_BODY: Code = Code::new("E0817");

    /// `spawn script … with(on: …)` written as anything but the word
    /// `"worker"` or the word `"here"` —
    /// `rule:concurrency/on-worker-runs-the-child-on-another-core`.
    ///
    /// The placement decides which scheduler starts the child and is decided
    /// once, where the spawn is written, so `on:` takes a written word rather
    /// than a `string` value. A computed placement could only be read at run
    /// time, where a spelling nobody declared has no answer but a spawn that
    /// fails — and a program that asked for another core and got a failure, or
    /// silently got this one, is measuring something it did not ask for. Two
    /// spawns under an `if` say the same thing with the question asked here.
    ///
    /// A code of its own rather than the mismatch against a union of the two
    /// literals that `nvs_types::expr::isolate` could report instead: an
    /// expected type claims a position accepts values of that type, and this
    /// one accepts two words.
    pub const E_SPAWN_PLACEMENT_UNKNOWN: Code = Code::new("E0818");

    /// A capture declared at an enum subset admits two cases carrying one
    /// written value —
    /// `rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name`.
    ///
    /// An alias is legal in the declaration (`rule:enums/declaration`: equality
    /// over an enum is value equality, so there is no identity for two names to
    /// collide on) and stays legal under the case-name spelling, where the names
    /// are still distinct. It is only the value spelling that cannot carry it:
    /// one segment would name two cases, and a match that picked either would be
    /// choosing for the program. Refused rather than first-wins, which is
    /// `rule:errors/ambiguous-input-refused`.
    pub const E_ROUTE_CAPTURE_CASES_SHARE_A_VALUE: Code = Code::new("E0819");

    /// A member that builds a written class out of a document — `decodeAs<T>`,
    /// `jsonAs<T>`, `queryAs<T>` on a request, `shapeAs<T>` — naming a class
    /// whose codec fills fewer constructor parameters than the constructor
    /// declares.
    ///
    /// `rule:core-classes/derive-field-list` sanctions `#[Json\Field(skip: true)]`
    /// on a property, and a skipped property that stayed a constructor
    /// parameter is the one way to leave a position no key fills. The class is
    /// well formed for every other purpose it has — it still encodes, and the
    /// program can still build one itself — so this is a refusal of the
    /// **call**, exactly as [`E_QUERY_AS_NOT_A_ROW_CLASS`] is at the row door
    /// and for the same reason: a condition about the class's own fields that
    /// cannot be moved to the declaration.
    ///
    /// The two doors keep separate codes because they refuse separate sets:
    /// a row is flat, so `E0806` also answers for a column map a document has
    /// no equivalent of.
    pub const E_DECODED_CLASS_NOT_CONSTRUCTIBLE: Code = Code::new("E0820");
    /// A member that builds an instance out of a document names a class that
    /// declared no JSON codec at all.
    ///
    /// `rule:core-classes/derive-attribute` makes participation opt-in, and a
    /// class takes one of two doors into it: `#[Json\Derive]`, which generates
    /// the `Core\Json\Codec`, or the `fromJson` half of that interface written
    /// by hand. A class that took neither can never be read out of a document,
    /// and the call naming it is where that is known — so it is refused while
    /// compiling rather than on the first request that reaches the member.
    /// [`E_QUERY_AS_NOT_A_ROW_CLASS`] is the same refusal at the row door.
    ///
    /// Separate from [`E_DECODED_CLASS_NOT_CONSTRUCTIBLE`] because the edit is
    /// a different one: that code names a contract the class does have and a
    /// constructor position it leaves unfilled, and this one names a class with
    /// no contract to inspect.
    pub const E_DECODED_CLASS_HAS_NO_CODEC: Code = Code::new("E0821");
    /// A written `return;` in a body whose declaration is not `void`.
    ///
    /// The defect [`E_MISSING_RETURN`] names, written out rather than reached
    /// by falling off the end: both exits lower to
    /// `nvs_ir::ir::Terminator::Return(None)`, so the caller of an `int`
    /// member reads a slot the callee never wrote. It is a code of its own
    /// because the edit is a different one — that code names a *path* an
    /// author has to find, and this one names the statement under the cursor.
    ///
    /// `void` is the whole of what makes one legal, which is why a
    /// constructor's is: it declares no return type at all, and
    /// [`E_CONSTRUCTOR_RETURN_CARRIES_A_VALUE`] is the other half of that
    /// pair. A generator's body reaches this nowhere — `rule:iteration/one-way-only` leaves it
    /// no return value to produce, so `nvs_types::check` checks it against
    /// `void` and `return;` is its only stop.
    pub const E_VALUELESS_RETURN: Code = Code::new("E0822");
    /// A method declaration that writes no return type, where the constructor
    /// is the one declaration allowed to.
    ///
    /// `rule:types/declaration` makes the slot mandatory, `void` and `never`
    /// included, and the constructor is the exception that rule states: it
    /// hands back the instance rather than a value, which is the same fact
    /// [`E_CONSTRUCTOR_RETURN_CARRIES_A_VALUE`] refuses a `return $x` in one
    /// for. Every other declaration owes a written type, because the
    /// declaration is what a caller reads and an omitted one means `mixed` —
    /// the widest type in the language, arrived at by leaving a slot empty
    /// rather than by writing it.
    ///
    /// Reported at a signature with no body too. An abstract method and an
    /// interface member are read by exactly the callers this is protecting, so
    /// the slot they leave empty is the one that costs most.
    /// [`E_CLOSURE_RETURN_TYPE_REQUIRED`] is this requirement at a
    /// block-bodied `fn`, and between them an expression-bodied closure is the
    /// only callable whose return type may go unwritten — its body is one
    /// expression, which is its own answer.
    pub const E_METHOD_RETURN_TYPE_REQUIRED: Code = Code::new("E0823");

    /// A `secret` value written into a container element or field whose own
    /// type does not carry the qualifier.
    ///
    /// `rule:security/secret-qualifier` puts the bit on a type, and a container
    /// keeps it only where the container's own element or field type spells it:
    /// an `array<T>` literal joins nothing, so `[$secret]` placed at an
    /// `array<mixed>` drops the qualifier at the bracket, and a shape literal's
    /// inferred field type drops it at the assignment into a field declared
    /// something wider. Refused at the write, which is the last place the
    /// qualifier is visible — `rule:errors/record-transformations`'s redaction
    /// row reads a *declared* type, so a secret that reaches a record through a
    /// container has nothing left to be redacted off.
    ///
    /// `rule:security/secret-sinks-refuse` names three positions a credential
    /// legitimately reaches — a bound database parameter, a process argv, an
    /// outbound request — and each is written as an argument, so an argument
    /// list is where this steps aside and the call's own rules decide.
    pub const E_SECRET_INTO_CONTAINER: Code = Code::new("E0824");

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
    /// A secret file the configuration reads can be read by an account other
    /// than the one the runtime runs as. `rule:config/a-secret-is-a-file-whose-content-is-the-value` advises here where it
    /// refuses on writability: a Compose secret is mounted `0444` and a
    /// Kubernetes secret volume defaults to `0644`, so inside a container this
    /// mode is the norm and on a shared host it is not, and nothing the
    /// runtime can read tells it which it is in.
    pub const W_SECRET_FILE_READABLE: Code = Code::new("W1005");
    /// `catch (Throwable) => value` — an expression-level arm naming the root
    /// of the exception tree, binding nothing, whose body is not a `throw`
    /// (`rule:expressions/bare-throwable-arm-warns`). In the block form a `catch (Throwable)` has a body with room to
    /// log or re-raise; in the expression form the body *is* the value, so an
    /// unbound arm over the root is by construction "discard every failure,
    /// including the ones this site never anticipated" — PHP's `@` operator,
    /// which `rule:php-migration/every-divergence-is-deliberate-and-listed`
    /// removed, regrown as a one-liner. The two honest spellings are naming
    /// the class the site expects and binding `$e` to carry the value. A
    /// warning rather than an error because the hazard is a habit and not a
    /// type error, the reason
    /// `rule:expressions/nullable-condition-lint`
    /// gives for its own.
    pub const W_CATCH_ARM_DISCARDS_EVERY_FAILURE: Code = Code::new("W1006");
    /// A credential the configuration puts in force begins or ends with
    /// whitespace. `rule:config/a-secret-is-a-file-whose-content-is-the-value` keeps such a value exactly as it was written —
    /// a password may legitimately carry an edge space, and removing it would
    /// be `rule:errors/ambiguous-input-refused`'s repair of input in place of a reading of it — so the value
    /// is used as-is and this is an advisory, never a refusal: refusing it
    /// would wall off a credential issued somewhere else, with no remedy in
    /// the file that names it.
    ///
    /// It is said out loud because that space is invisible in the one place an
    /// operator looks. In the file half of a § 7 pair there is nothing to see
    /// at all, and at the end of a TOML line it is a space before a quote. A
    /// reader that quietly trims it instead turns a working credential into an
    /// authentication failure at the far end, which is the bug this code
    /// exists to make loud.
    pub const W_CREDENTIAL_HAS_EDGE_WHITESPACE: Code = Code::new("W1007");
    /// The configuration names a shared store and grants nothing that may
    /// reach it: `[cache.shared] url` is set and `cache.shared` is not
    /// granted, so `Core\Cache::shared()` and `Core\RateLimit::consume` would
    /// both be refused at the door.
    /// `rule:config/cache-shared-is-the-grant-over-the-configured-store` is
    /// what makes that pair legible at boot — the two keys are in the same
    /// file, so the deployment that granted `net.connect` for its store's host
    /// hears about it while an operator is reading output rather than on the
    /// first request that touches the tier.
    ///
    /// An advisory and not a refusal, because the pair is not wrong on its
    /// own: a fleet may share one `nvs.toml` between an application that uses
    /// the tier and one that does not, and the second is a program with
    /// nothing to fix. What it may not be is *silent*, since the failure it
    /// otherwise produces arrives one deploy later and names only the member.
    pub const W_STORE_CONFIGURED_UNGRANTED: Code = Code::new("W1008");
    /// `[http.client.tls] keylog` is on, and this host is one where that is
    /// allowed: every outbound TLS session is appending its secrets to the
    /// named file.
    ///
    /// Announced at every start rather than once, because the state it
    /// describes is one somebody switched on for an afternoon's debugging and
    /// nobody reports having switched off — a capture left in place is the
    /// deployment's whole outbound traffic readable by anyone who can read a
    /// file. It is not a refusal here for the reason it is `E0640` elsewhere:
    /// `production` never reaches this, so the only host that sees it has
    /// already said it is being debugged.
    pub const W_TLS_KEYLOG_ON: Code = Code::new("W1009");
    /// `[http.client.proxy] resolve` is `proxy`, so the destination's
    /// address is the proxy's question and no longer this deployment's.
    ///
    /// `rule:security/net-address-policy` is not being violated — the
    /// operator has said in configuration that the address question has
    /// moved — but it is a real narrowing: Novis judges the scheme, the
    /// `net.connect` host list and the tainted-URL check, and never learns
    /// an address to judge. Written at every start rather than once,
    /// because a deployment that has run this way for a year still owes
    /// today's log the sentence, and the alternative is an auditor reading
    /// a boot record that says the address policy is in force when what is
    /// in force is the proxy's.
    pub const W_PROXY_RESOLVES_THE_DESTINATION: Code = Code::new("W1010");
}

#[cfg(test)]
mod tests {
    use super::{Code, code};

    /// Every `E01xx` number this file declares, read out of the registry's own
    /// source. The escaped quote in the pattern below is what keeps this
    /// function from matching itself.
    fn parser_band() -> Vec<u32> {
        include_str!("lib.rs")
            .split("Code::new(\"E01")
            .skip(1)
            .filter_map(|rest| rest.get(..2))
            .map(|digits| digits.parse::<u32>().expect("a code is four digits"))
            .collect()
    }

    /// The pipeline's three sit one past the highest code that predates them,
    /// rather than in the holes at `E0124`–`E0126` that ADR 0098 named before
    /// those numbers were taken. That is what an allocation owes
    /// `conventions.md` § *A diagnostic*: the band's highest plus one, never
    /// its lowest hole. The claim is about where the three were allocated, so
    /// it holds whatever is allocated above them afterwards — which is the
    /// separate invariant the test below carries.
    #[test]
    fn the_pipeline_codes_are_the_next_free_parser_band_numbers() {
        let pipeline = [
            code::E_PIPELINE_RIGHT_SIDE_HAS_NO_HOLE,
            code::E_PIPELINE_RIGHT_SIDE_REPEATS_THE_HOLE,
            code::E_HOLE_OUTSIDE_A_PIPELINE,
        ];
        assert_eq!(pipeline.map(Code::as_str), ["E0129", "E0130", "E0131"]);

        assert_eq!(
            parser_band().into_iter().filter(|n| *n < 29).max(),
            Some(28),
            "the pipeline's three no longer follow on from the band's highest, \
             so they read as filling a hole rather than as the next free numbers",
        );
    }

    /// The band's highest number is the newest code in it, which is what makes
    /// "the next free one is the highest plus one" answerable by reading the
    /// registry. A code allocated into a hole would leave this failing rather
    /// than leave `brief.py`'s next-free line quietly wrong.
    #[test]
    fn the_newest_parser_code_is_the_bands_highest_number() {
        assert_eq!(
            code::E_TYPE_ALIAS_TAKES_NO_MODIFIER_OR_ATTRIBUTE.as_str(),
            "E0133"
        );

        let mut band = parser_band();
        assert!(band.len() > 3, "the band did not parse: {band:?}");
        band.sort_unstable();
        let mut unique = band.clone();
        unique.dedup();
        assert_eq!(band, unique, "two parser-band codes share a number");
        assert_eq!(
            band.pop(),
            Some(33),
            "the newest code is no longer the band's highest, so the number after \
             it is no longer the next free one",
        );
    }

    #[test]
    fn the_codes_adr_0098_originally_named_still_belong_to_their_owners() {
        assert_eq!(code::E_FOR_INIT_MIXES_DECL_AND_EXPR.as_str(), "E0124");
        assert_eq!(code::E_FOR_INIT_TWO_DECLARATIONS.as_str(), "E0125");
        assert_eq!(code::E_CATCH_ARM_NOT_AN_EXPRESSION.as_str(), "E0126");
    }
}
