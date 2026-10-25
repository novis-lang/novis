//! The parser's codes, `E01xx`.
//!
//! A new code here takes the highest number in this file plus one, never a
//! hole: a hole is a retired number. A code outside `E01xx` fails the
//! crate's tests.

codes! {
    range = "E01";

    /// A specific token was required here.
    pub const E_EXPECTED_TOKEN: Code = Code::new("E0101").card(
        "A specific token is missing here, such as a `;`, a `)` or a type. The message names \
         the token that is needed. Add it, or check the line before this one for a mistake.",
    );
    /// An expression was required here.
    pub const E_EXPECTED_EXPR: Code = Code::new("E0102").card(
        "An expression is needed here, such as a value, a variable or a function call. Add \
         one, or check the line for a missing or extra operator.",
    );
    // `E0103` is retired and is never reused: the parser reads a statement
    // position as an expression statement, so `E0101` and `E0102` report it.
    // `E0104` is retired and is never reused: the parser reports a bracket
    // that is never closed as the token it expected, `E0101`, and no check
    // raised it.
    /// Assignment to something that cannot be assigned to.
    pub const E_INVALID_ASSIGN_TARGET: Code = Code::new("E0105").card(
        "This expression cannot be assigned to. The left side of `=` must be something that \
         can store a value, such as a variable, a property or an array element.",
    );
    /// A modifier that is not allowed in this position.
    pub const E_BAD_MODIFIER: Code = Code::new("E0106").card(
        "This modifier is not allowed in this position. Delete it, or move it to a declaration \
         that allows it.",
    );
    /// A parameter list with two parameters of one name, or with a `...` on a
    /// parameter that is not the last.
    pub const E_BAD_PARAM_LIST: Code = Code::new("E0107").card(
        "This parameter list is not valid. Each parameter needs its own name, and only the last          parameter can be variadic (written with `...`). Rename the parameter, or move the          variadic parameter to the end of the list.",
    );
    /// Recursive-descent parsing nested past the recursion limit — malformed
    /// or adversarial input (e.g. thousands of nested `[`), never legitimate
    /// source. The parser bails out here rather than overflowing its stack.
    pub const E_TOO_DEEPLY_NESTED: Code = Code::new("E0108").card(
        "The code is nested too deeply, so the compiler stopped reading it. Real programs do \
         not reach this limit. Check the file for many brackets that are opened and never closed.",
    );
    /// `tainted` applied to anything other than `string`/`bytes` — the
    /// qualifier's grammar restricts it to those two scalars; see
    /// `rule:security/tainted-qualifier`.
    pub const E_TAINTED_NON_SCALAR: Code = Code::new("E0109").card(
        "`tainted` can only be written before `string`, `bytes` or a shape that contains them. \
         A tainted value is text that came from outside the program, such as a request. Write \
         `tainted string`, `tainted bytes` or `tainted {…}`.",
    );
    /// A class/interface/trait/enum/enum-case/namespace-segment name is not
    /// `PascalCase` — `rule:core-api/identifier-casing`'s casing table.
    pub const E_BAD_TYPE_CASING: Code = Code::new("E0110").card(
        "This name must be written in PascalCase, for example `OrderItem`. Names of classes, \
         interfaces, traits, enums, enum cases and namespaces all use this style. Rename it to \
         the name the message suggests.",
    );
    /// A method name is not `camelCase` — `rule:core-api/identifier-casing`'s casing table. Distinct
    /// from `E_LEGACY_CONSTRUCTOR_SPELLING`, which covers the one mis-cased
    /// spelling (`__construct`) that gets a targeted fix instead of this
    /// generic diagnostic.
    pub const E_BAD_METHOD_CASING: Code = Code::new("E0111").card(
        "A method name must be written in camelCase, for example `findOrder`. Rename it to the \
         name the message suggests.",
    );
    /// A property, parameter, local variable or anonymous function's self-name is not
    /// `camelCase` — `rule:core-api/identifier-casing`'s casing table, tightened by
    /// `rule:classes/no-leading-underscore-identifiers`, which allows no leading
    /// underscore at all.
    pub const E_BAD_MEMBER_CASING: Code = Code::new("E0112").card(
        "This name must be written in camelCase, for example `$orderId`. Properties, parameters \
         and variables use this style, and a name cannot start with `_`. Rename it to the name \
         the message suggests.",
    );
    /// A class constant name is not `SCREAMING_SNAKE_CASE` — `rule:core-api/identifier-casing`'s
    /// casing table.
    pub const E_BAD_CONST_CASING: Code = Code::new("E0113").card(
        "A class constant name must be written in SCREAMING_SNAKE_CASE, for example `MAX_SIZE`. \
         Rename it to the name the message suggests.",
    );
    /// A method literally named `__construct` —
    /// `rule:classes/constructor-is-a-method-named-constructor`: Novis's
    /// constructor is spelled `constructor`, an ordinary `camelCase` method
    /// name needing no exception of its own.
    pub const E_LEGACY_CONSTRUCTOR_SPELLING: Code = Code::new("E0114").card(
        "In Novis, the constructor method is named `constructor`. Rename `__construct` to \
         `constructor`.",
    );
    /// `secret` applied to anything other than `string`/`bytes` — the
    /// qualifier's grammar restricts it to those two scalars, the same
    /// restriction `E_TAINTED_NON_SCALAR` enforces for `tainted`; see
    /// `rule:security/secret-qualifier`.
    pub const E_SECRET_NON_SCALAR: Code = Code::new("E0115").card(
        "`secret` can only be written before `string` or `bytes`. A secret value is text such \
         as a password or a key. Write `secret string` or `secret bytes`.",
    );
    /// `tainted secret string`/`tainted secret bytes`: `secret` and `tainted`
    /// compose, but only in the order `secret` before `tainted` — see
    /// `rule:security/secret-qualifier`.
    pub const E_SECRET_TAINTED_ORDER: Code = Code::new("E0116").card(
        "`secret` must come before `tainted`. Write `secret tainted string` or \
         `secret tainted bytes`.",
    );
    /// A `{ name: value, ... }` anonymous object written where `{` already
    /// commits to a block — an expression-bodied `fn() => {...}`, or a bare
    /// statement-initial `{...}` — needs the same parenthesize-to-force-
    /// expression fix JavaScript uses for the identical ambiguity; see
    /// `rule:types/anonymous-object`.
    pub const E_ANON_OBJECT_NEEDS_PARENS: Code = Code::new("E0117").card(
        "An anonymous object in this position is read as a block of code. This happens after \
         `fn() =>` and at the start of a statement. Put parentheses around it: \
         `({ name: value })`.",
    );
    /// `{x}` — an anonymous object has no shorthand; every field is written
    /// `name: value`. See `rule:types/anonymous-object`.
    pub const E_ANON_OBJECT_SHORTHAND: Code = Code::new("E0118").card(
        "An anonymous object has no short form for a field. Write each field as `name: value`. \
         For example, write `{ x: $x }`, not `{x}`.",
    );
    /// `{[$expr]: value}` — an anonymous object has no computed/dynamic key;
    /// every field name is a static identifier. See `rule:types/anonymous-object`.
    pub const E_ANON_OBJECT_COMPUTED_KEY: Code = Code::new("E0119").card(
        "A field name in an anonymous object cannot be computed from an expression. Write each \
         field name directly, for example `{ name: value }`.",
    );
    /// A `float` literal in type position — `rule:types/single-value-types` defers single-value
    /// `float` types until floating-point equality has a real answer, so `0.1` names
    /// no type the way `1` and `"a"` do.
    pub const E_FLOAT_SINGLE_VALUE_TYPE: Code = Code::new("E0120").card(
        "A `float` value such as `0.1` cannot be used as a type. Use the type `float` and check \
         the value in your code. Or use `int` values as the type, such as `1|2|3`.",
    );
    /// An interpolated string in type position — `"a"` is `rule:types/single-value-types`'s
    /// singleton type, and a type has no scope to interpolate a variable
    /// from.
    pub const E_INTERPOLATION_IN_TYPE: Code = Code::new("E0121").card(
        "A string used as a type cannot contain a variable. A type is fixed when the program \
         compiles. Write the string out in full, for example `\"draft\"`.",
    );
    /// A member declaration — property, class constant or method, in a
    /// `class`, `interface` or anonymous-class body — carrying no
    /// `public`/`protected`/`private`, or PHP 8.4's `(set)` form written
    /// without its read visibility. There is no implicit `public`; see
    /// `rule:core-api/written-visibility`.
    /// A class body's PHP `var $x;` reports this too, rather than a message
    /// about the statement grammar it would otherwise fall into (§ 4).
    pub const E_MISSING_VISIBILITY: Code = Code::new("E0122").card(
        "Every property, constant and method in a class must say `public`, `protected` or \
         `private`. Nothing is public by default, and `var` is not allowed. Write, for example, \
         `public int $count;`.",
    );
    /// An `autoload` prefix, root or glob written as anything but a plain
    /// string literal — an interpolated `"$dir"`, a concatenation, a
    /// variable. Every path resolves at compile time, relative to the file
    /// the declaration appears in, so there is nothing to interpolate from;
    /// see `rule:programs/autoload`, which carries `require`'s string-literal-only restriction for
    /// the same reason.
    pub const E_AUTOLOAD_PATH_NOT_WRITTEN_DIRECTLY: Code = Code::new("E0123").card(
        "An `autoload` path must be a plain string, such as `\"src/\"`. It cannot contain a \
         variable or be built from parts. The path is read when the program compiles, relative \
         to this file.",
    );
    /// A `for` init clause holding a declaration *and* an expression, in
    /// either order — `for (int $i = 0, $j = 1; …)` and
    /// `for ($j = 1, int $i = 0; …)` alike. See
    /// `rule:iteration/for-init-refusals`, whose § 1 makes the clause one or the other and never both.
    pub const E_FOR_INIT_MIXES_DECL_AND_EXPR: Code = Code::new("E0124").card(
        "The first part of a `for` loop can declare one variable or list expressions, but not \
         both. Move the extra part above the loop, or make every item an expression.",
    );
    /// A `for` init clause holding two declarations, `for (int $i = 0, int
    /// $j = 0; …)` — the shape a reader coming from C writes. Separate from
    /// [`E_FOR_INIT_MIXES_DECL_AND_EXPR`] because the fix is different:
    /// the second declaration goes above the loop. `rule:iteration/for-init-refusals`.
    pub const E_FOR_INIT_TWO_DECLARATIONS: Code = Code::new("E0125").card(
        "The first part of a `for` loop can declare only one variable. Declare the second \
         variable above the loop. For example, write `int $j = 0;` and then \
         `for (int $i = 0; …)`.",
    );
    /// `return`, `break` or `continue` as the body of an expression-level
    /// `catch` arm. See
    /// `rule:expressions/catch-arm-is-an-expression`: an arm holds an expression, which admits `throw` — already an
    /// expression — and refuses the three that are statements. Named rather
    /// than left to the generic expected-expression error, because the fix is
    /// a different construct and not a different token.
    pub const E_CATCH_ARM_NOT_AN_EXPRESSION: Code = Code::new("E0126").card(
        "A `catch` arm after `=>` must be an expression, so `return`, `break` and `continue` \
         are not allowed there. `throw` is allowed. For the other three, write the block form \
         of `catch` with `{ … }`.",
    );
    /// A `///` run with no declaration under it —
    /// `rule:tooling/doc-comment-attaches-to-the-next-declaration`. Reported
    /// rather than ignored because the marker is what separates documentation
    /// from a note to self: a `///` that documents nothing is either a note
    /// written with the wrong marker or a declaration that got deleted out from
    /// under it, and both are worth saying. The fix is one slash fewer.
    pub const E_DOC_COMMENT_UNATTACHED: Code = Code::new("E0127").card(
        "This doc comment (`///`) has no declaration under it. A doc comment must be directly \
         above a declaration, with no blank line between them. For a normal note, write `//`.",
    );
    /// An `@tag` at the start of a doc comment line that is neither `@see` nor
    /// `@example` — `rule:tooling/doc-comment-tags-are-see-and-example`. One
    /// code for every rejected spelling, with the help naming what to write
    /// instead, because the answer for `@param` and the answer for an invented
    /// tag differ in wording and not in kind: the set is closed, and this
    /// diagnostic is the entire difference between a closed set and a
    /// convention.
    pub const E_DOC_COMMENT_UNKNOWN_TAG: Code = Code::new("E0128").card(
        "A doc comment allows only two tags: `@see` and `@example`. Delete this tag, and write \
         what it says as normal text.",
    );
    /// The right side of a `|>` with no `$_` in it — the help names the shape
    /// (`Str::trim($_)`), and when the right side is a method reference
    /// or a callable value it adds that PHP 8.5's `|>` applies a
    /// callable where this one substitutes a hole.
    /// `rule:expressions/pipeline-hole-once`: the whole affordability of
    /// sharing the spelling with PHP is that the habit is refused at the
    /// character where it goes wrong rather than meaning something else.
    pub const E_PIPELINE_RIGHT_SIDE_HAS_NO_HOLE: Code = Code::new("E0129").card(
        "The right side of `|>` must contain `$_`. `$_` is replaced by the value on the left. \
         Write `$_` where the value goes, for example `$name |> Str::trim($_)`.",
    );
    /// `$_` more than once on one right side of a `|>`, whose help is to bind
    /// the value to a local instead. Exactly one hole is what makes
    /// `rule:expressions/pipeline-substitution` a substitution with no
    /// temporary and no double evaluation, so this is a refusal and not a
    /// second lowering.
    pub const E_PIPELINE_RIGHT_SIDE_REPEATS_THE_HOLE: Code = Code::new("E0130").card(
        "`$_` can appear only once on the right side of `|>`. To use the value twice, store it \
         in a variable first.",
    );
    /// `$_` anywhere that is not the right side of a `|>`, where it names
    /// nothing — `rule:expressions/pipeline-hole-once`. Separate from the
    /// unresolved-variable error because the fix is a `|>` and not a
    /// declaration.
    pub const E_HOLE_OUTSIDE_A_PIPELINE: Code = Code::new("E0131").card(
        "`$_` has a meaning only on the right side of `|>`. There it is replaced by the value \
         on the left. Use `|>` here, or give the variable another name.",
    );
    /// `tainted {…}` over a shape carrying no `string` and no `bytes` anywhere
    /// — `rule:security/tainted-qualifier`. The qualifier distributes to text
    /// and there is none to reach, so it promises something nothing enforces,
    /// which is worth a diagnostic rather than the silent no-op it would
    /// otherwise be.
    pub const E_TAINTED_SHAPE_HAS_NO_TEXT: Code = Code::new("E0132").card(
        "`tainted` has no effect on this shape, because the shape has no `string` or `bytes` \
         field. Delete `tainted`, or add the text field the shape should have.",
    );
    /// A modifier run or an attribute group written in front of a body's
    /// `type` alias — `rule:types/type-alias`. An alias is reachable wherever
    /// its owner's name is, so there is no visibility to write, and nothing
    /// downstream of the checker ever sees the name, so an attribute has
    /// nothing to attach to. Parsing the run and dropping it would accept a
    /// declaration saying something the language does not have.
    ///
    /// An `enum` in a class body gets the same code for a modifier run, for
    /// the same reason (`rule:enums/class-scoped-enum`), and keeps its
    /// attributes.
    pub const E_TYPE_ALIAS_TAKES_NO_MODIFIER_OR_ATTRIBUTE: Code = Code::new("E0133").card(
        "A `type` alias cannot have a modifier such as `public`, or an attribute. Delete them. \
         An enum in a class cannot have a modifier either. Both can be used wherever their \
         class can be used.",
    );
    /// A bare shape type written in front of a local's name (`{x: int}
    /// $point;`) — `rule:types/shape-type`. A statement-initial `{` is a block
    /// before it is anything else, so the shape never reaches a declaration
    /// there; every other slot takes a bare shape fine, and the help names the
    /// `type` alias that rule's own example uses. Reported in place of
    /// [`E_ANON_OBJECT_NEEDS_PARENS`] when the token after the matched `}`
    /// is a variable, because parentheses are not the fix for this one.
    pub const E_SHAPE_TYPED_LOCAL_NEEDS_AN_ALIAS: Code = Code::new("E0134").card(
        "A variable cannot be declared with a shape type written directly in front of it. Give \
         the shape a name first, for example `type Point = { x: int };`. Then write \
         `Point $point;`.",
    );
    /// A `::` written after `self`, `static` or `parent` in type position
    /// (`self::MAX $n;`, `$x is self::Wild`) — `rule:types/grammar`. The three
    /// keywords are whole atoms there, and every member a `::` could reach —
    /// a `type` alias, a class constant, an enum case — is already spelled
    /// with the owner's own name, an alias additionally with its bare name
    /// inside the body that declares it. The keyword form would be a second
    /// name for each of them, and `static::` could only ever mean the lexical
    /// class, which the written name says
    /// (`rule:types/class-scoped-alias` § *Two spellings and no third*).
    pub const E_CLASS_KEYWORD_MEMBER_IN_TYPE: Code = Code::new("E0135").card(
        "`self::`, `static::` and `parent::` cannot be used in a type. Write the class name \
         instead, for example `Order::Status`. Inside the class that declares a `type` alias \
         or an enum, you can also write its name alone.",
    );
}
