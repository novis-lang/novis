//! The type checker's codes, `E04xx`.
//!
//! The range is full at `E0499`, so a new code of this area goes in
//! `types_e08.rs`.

codes! {
    range = "E04";

    /// A value whose type cannot be what this position requires.
    pub const E_TYPE_MISMATCH: Code = Code::new("E0401").card(
        "A value has a type that this place does not accept. Change the value, or convert it with \
         `as` to the type that is needed.",
    );
    /// Wrong number of arguments — including a **shape key** a member
    /// requires and a written anonymous object does not carry. `rule:core-api/shape-flattens-at-the-abi` flattens
    /// each key of a shape parameter into one argument of its own, so an
    /// omitted required key is a call one argument short rather than a
    /// separate kind of mistake.
    pub const E_ARITY_MISMATCH: Code = Code::new("E0402").card(
        "A call passes the wrong number of arguments. Check the parameters of the method. A shape \
         argument must also contain every key that the method needs.",
    );
    /// A return type that no return statement can satisfy.
    pub const E_BAD_RETURN_TYPE: Code = Code::new("E0403").card(
        "A method returns a value of a different type than its declared return type. Change the \
         return type, or change what the method returns.",
    );
    /// An override whose signature is not compatible with the parent's.
    ///
    /// `rule:classes/an-override-keeps-its-parents-signature`:
    /// `nvs_types::conformance::check_method_overrides` reports the return
    /// type, each parameter, the count, `static` and the visibility, and
    /// `reject_dropped_static_return` a `: static` an interface's implementor
    /// drops.
    pub const E_INCOMPATIBLE_OVERRIDE: Code = Code::new("E0404").card(
        "This method replaces a method of a parent class or an interface, and a call through the \
         parent cannot use it. Keep the parameters, `static` and the visibility. A parameter may \
         accept a wider type, and the return type may be narrower. A parameter with a narrower \
         type needs `covariant` before its type.",
    );
    /// A property or method access on a type that has no such member.
    pub const E_UNKNOWN_MEMBER: Code = Code::new("E0405").card(
        "The type of this value has no property or method with this name. Check the name for a \
         typo, and check the type of the value.",
    );
    /// A local variable declared a second time while its first declaration
    /// is still live — `rule:types/declaration`: "there is no shadowing."
    pub const E_REDECLARED_LOCAL: Code = Code::new("E0406").card(
        "This variable is already declared, and Novis does not allow a second declaration of the \
         same variable. Assign to the existing variable, or use a different name.",
    );
    /// `int ⊕ uint` arithmetic — `rule:types/arithmetic`: there is no representable
    /// common type, so one side must be converted explicitly.
    pub const E_INT_UINT_ARITHMETIC: Code = Code::new("E0407").card(
        "Arithmetic between an `int` and a `uint` is not allowed, because no type can hold every \
         result. Convert one side first with `as int` or `as uint`.",
    );
    /// A type nests past `rule:types/arrays`'s depth-32 bound.
    pub const E_ARRAY_TYPE_TOO_DEEP: Code = Code::new("E0408").card(
        "A type is nested more than 32 levels deep, which is the limit. An `array<...>`, a shape \
         and a `callable(...)` signature each add one level. Use a class for some of the inner \
         levels.",
    );
    /// A non-nullable, no-default property a class declares (itself, or
    /// through a used trait) is not definitely assigned on some path out of
    /// its constructor — or the class has no constructor at all to assign
    /// it, or the constructor lets `$this` escape before it is assigned; see
    /// `rule:classes/definite-property-initialization`.
    pub const E_UNINITIALIZED_PROPERTY: Code = Code::new("E0409").card(
        "A property with no default value and no `?` in its type is not assigned on every path \
         through the constructor. The constructor must also assign it before it uses `$this` or \
         calls `parent::constructor(...)`. Assign it earlier, or give it a default value. If the \
         value needs `$this`, declare the property `lateinit` and assign it after the \
         constructor. A class that implements `LateInit` sets a `lateinit` property in its \
         `initProperty` method, the first time the property is read.",
    );
    /// A subclass constructor has a path that never calls
    /// `parent::constructor(...)`, so the properties it inherits are never
    /// discharged on that path; see `rule:classes/definite-property-initialization`.
    pub const E_MISSING_PARENT_CONSTRUCTOR_CALL: Code = Code::new("E0410").card(
        "The constructor of a class that `extends` another class must call \
         `parent::constructor(...)` on every path. Add the call where it is missing.",
    );
    /// `<`/`>`/`<=`/`>=`/`<=>` between two objects whose static types are not
    /// both provably the same class implementing the reserved global
    /// `Comparable` interface — either one side doesn't implement it, or the
    /// two sides are different classes even though both do; see `rule:classes/comparable` and `rule:classes/comparable-is-same-class-only`. PHP's implicit property-walk fallback has no Novis equivalent.
    pub const E_COMPARISON_REQUIRES_COMPARABLE: Code = Code::new("E0411").card(
        "`<`, `>`, `<=`, `>=` and `<=>` compare two objects only when both have the same class, \
         and that class implements `Comparable`. Implement `compareTo(self $other): int` on the \
         class, or compare a property instead.",
    );
    /// An object used at an implicit string-conversion site (interpolation,
    /// concatenation, `echo`/`print`, `as string`/`(string)`) whose static
    /// type does not provably implement the reserved global `Stringable`
    /// interface; see `rule:classes/stringable`. PHP's own fallback here is already a
    /// fatal error, so nothing permissive is being removed.
    pub const E_STRINGABLE_REQUIRED: Code = Code::new("E0412").card(
        "An object is used as a string, but its class does not implement `Stringable`. Implement \
         `toString(): string` on the class, or use a string property instead.",
    );
    /// `unset()` on a declared property, static or instance, regardless of
    /// nullability — refused outright because `rule:classes/definite-property-initialization` already guarantees no
    /// declared property is ever anything but definitely initialized; see
    /// `rule:classes/unset-is-refused-on-a-property`. A static property is the same slot and the same guarantee, so
    /// it takes the same code, named for the class that *declares* it. Every
    /// other operand `unset()` cannot remove an entry from is
    /// [`E_UNSET_TARGET_NOT_AN_ELEMENT`].
    pub const E_UNSET_ON_PROPERTY: Code = Code::new("E0413").card(
        "`unset()` does not work on a declared property, because a property always has a value. \
         If the type of the property is nullable, assign `null` instead.",
    );
    /// `var $x = [...];`, or `foreach ([...] as var $v)`, over a bare array
    /// literal whose elements do not all have one type: two or more types, an
    /// empty literal at any depth, or an element that could not be typed. See
    /// `rule:types/var-inference`.
    pub const E_VAR_ARRAY_LITERAL_NEEDS_TYPE: Code = Code::new("E0414").card(
        "`var` finds the type of an array only when every element has the same type. Here the \
         elements have different types, or the array is empty. Write the type instead, as in \
         `array<int|string> $row = [1, \"a\"];`. In a `foreach`, put the array in a variable \
         like this first, and loop over the variable.",
    );
    /// An arithmetic or bitwise operator — binary, prefix `-`, `+` and `~`, or
    /// `++`/`--` — whose operand can be an enum case: an enum, a case-subset
    /// type, or a union with either in it, such as `?Size`. A case is not a
    /// number; `as int`/`as uint`, or `as ?int` for a nullable value, gives
    /// the integer behind it. See `rule:types/arithmetic`.
    pub const E_ENUM_ARITHMETIC_UNSUPPORTED: Code = Code::new("E0415").card(
        "Arithmetic and bitwise operators do not work on an enum value. Convert it to its number \
         first, as in `$level as int`.",
    );
    /// `as` from one enum type to a *different* enum type, even when both
    /// share the same underlying integer type — rejected outright; an
    /// explicit `match` naming every case is the replacement. See `rule:types/conversion`.
    pub const E_ENUM_CONVERSION_UNSUPPORTED: Code = Code::new("E0416").card(
        "`as` cannot convert a value of one enum type to a different enum type. Write a `match` \
         that gives a case of the new enum for every case of the old one.",
    );
    /// `as Core\Html\Markup` on anything but a string literal. A
    /// runtime-computed or `tainted` value can never become a trusted
    /// `Markup` this way, which closes "compute the escape-defeating payload
    /// at runtime, then convert it." See `rule:core-classes/html-auto-escape`.
    pub const E_MARKUP_NEEDS_WRITTEN_STRING: Code = Code::new("E0417").card(
        "`as Core\\Html\\Markup` works only on a string written directly in the source code. A \
         value computed while the program runs cannot become trusted HTML this way. Build the \
         markup from strings written in the code, or escape the value with a `Core\\Html` \
         method.",
    );
    /// A string passed (or convertible without laundering) where `callable`
    /// is the declared type — PHP's bare-name/`"Class::method"` callable
    /// spellings are both rejected in favor of a method reference.
    /// See `rule:types/callable-values`.
    pub const E_CALLABLE_STRING_UNSUPPORTED: Code = Code::new("E0418").card(
        "A string cannot be called as a function, even if it contains the name of one. Write \
         `Class::method` or `$obj->method(...)` to get a callable.",
    );
    /// A `[$obj, 'method']`-shaped array passed where `callable` is the
    /// declared type. See `rule:types/callable-values`.
    pub const E_CALLABLE_ARRAY_UNSUPPORTED: Code = Code::new("E0419").card(
        "An array such as `[$obj, 'method']` cannot be called as a function. Write \
         `$obj->method(...)` to get a callable.",
    );
    /// `$obj(...)` where `$obj`'s static type is not `callable` — Novis has no
    /// `__invoke`, so no class ever makes `()` mean anything else. See
    /// `rule:types/callable-values`.
    pub const E_NOT_CALLABLE: Code = Code::new("E0420").card(
        "Only a `callable` can be called with `(...)`, and this value is not one. Write an \
         anonymous function with `fn`, or get a callable with `Class::method` or \
         `$obj->method(...)`. An object cannot be called, because Novis has no `__invoke`.",
    );
    /// A `secret`-qualified value reaching a `Core\Html\Markup`-building
    /// conversion — refused even though the equivalent `tainted`-only value
    /// would (once `Core\Html` exists) be auto-escaped instead: escaping
    /// neutralizes injection risk, not confidentiality. See `rule:security/secret-sinks-refuse`.
    pub const E_SECRET_MARKUP_UNSUPPORTED: Code = Code::new("E0421").card(
        "A `secret` value, such as a password or a key, cannot be converted to HTML markup. \
         Escaping protects against injection, but it does not keep the value private. If you \
         really want to show it, call `Core\\Secret::reveal(...)` first.",
    );
    /// A `secret`-qualified value passed as a `Throwable`-shaped class's
    /// constructor message argument — closing the common leak of a
    /// credential ending up in a stack trace or an error page. See `rule:security/secret-sinks-refuse`
    /// .
    pub const E_SECRET_THROWABLE_MESSAGE: Code = Code::new("E0422").card(
        "A `secret` value, such as a password or a key, cannot be the message of an exception. \
         The message can appear in a stack trace or on an error page. If you really want to show \
         it, call `Core\\Secret::reveal(...)` first.",
    );
    /// The `parent` type atom (`parent $x`, a parameter/property/return
    /// position — distinct from `new parent(...)`, which silently falls back
    /// to `mixed` for the same shape) used in a class with no `extends`.
    pub const E_NO_PARENT_CLASS: Code = Code::new("E0423").card(
        "The type `parent` is used in a class that does not extend another class. Write the name \
         of the class you mean, or add `extends` to the class.",
    );
    // `E0424` and `E0425` are retired and are never reused: they refused
    // `lateinit` on a type that is not a class or an interface, and on a `?T`.
    // `lateinit` takes every type (`rule:classes/lateinit-restrictions`).
    /// `lateinit` on a promoted constructor parameter — binding the
    /// parameter is already the assignment `rule:classes/definite-property-initialization` requires, so there is
    /// nothing left to defer. See `rule:classes/lateinit-restrictions`.
    pub const E_LATEINIT_PROMOTED_PARAM: Code = Code::new("E0426").card(
        "`lateinit` cannot be used on a constructor parameter. The constructor already assigns \
         the parameter, so there is nothing to set later. Remove `lateinit`.",
    );
    /// `lateinit` combined with `readonly` on the same property — opposite
    /// promises about when the one allowed assignment happens. See `rule:classes/lateinit-restrictions`.
    pub const E_LATEINIT_READONLY_CONFLICT: Code = Code::new("E0427").card(
        "`lateinit` and `readonly` cannot be used on the same property. A `readonly` property is \
         set in the constructor, and a `lateinit` property is set after it. Choose one of them.",
    );
    /// A `lateinit` property read inside a method body with no intervening
    /// write to it and no intervening call on that path since the method's
    /// entry — the one intraprocedural, false-positive-free case `rule:classes/lateinit-read-before-write` proves at compile time; every other case relies entirely on the
    /// runtime throw `rule:classes/lateinit` defines.
    pub const E_LATEINIT_READ_BEFORE_WRITE_LOCAL: Code = Code::new("E0428").card(
        "A method reads a `lateinit` property before anything in the method assigns it. Assign \
         the property before this line, or read it after the call that sets it. You can also \
         implement `LateInit` in the class. Its `initProperty` method then sets the property the \
         first time it is read.",
    );
    /// An integer literal whose magnitude doesn't fit the width it's being
    /// checked against — too large for `int`/`uint` outright, or exactly the
    /// one magnitude `uint` can never represent regardless of width: a
    /// negative value, since an integer literal's digits are never signed and
    /// the sign comes from a wrapping unary `-`. See `rule:types/arithmetic`.
    pub const E_INT_LITERAL_OUT_OF_RANGE: Code = Code::new("E0429").card(
        "This integer is too large for its type, or it is negative where a `uint` is needed. A \
         value too large for `int` is allowed only where a `uint` is expected.",
    );
    /// A `\u{...}` escape inside a double-quoted string literal (or an
    /// interpolated-heredoc text run) names a value outside Unicode's valid
    /// scalar range (`> 0x10FFFF`) or inside the UTF-16 surrogate range
    /// (`0xD800..=0xDFFF`) — neither has a UTF-8 encoding, so it cannot be
    /// cooked into `string`'s guaranteed-valid-UTF-8 representation. Distinct
    /// from `E_INVALID_ESCAPE`, which the lexer already raises for a
    /// `\u{...}` that is syntactically malformed (no hex digits, or no
    /// closing `}`) — this fires only once the digits are syntactically fine
    /// but numerically out of range.
    pub const E_INVALID_UNICODE_ESCAPE: Code = Code::new("E0430").card(
        "A `\\u{...}` escape in a string names a number that is not a valid Unicode character. \
         The number must be at most `10FFFF`, and it must not be between `D800` and `DFFF`.",
    );
    /// A double-quoted string literal's (or interpolated-heredoc text run's)
    /// `\xHH`/octal byte escapes assembled into a sequence that is not valid
    /// UTF-8 — `string` is guaranteed-valid UTF-8 (`rule:types/bytes`), so a byte
    /// escape's raw output has to actually decode, not just fit in a byte.
    pub const E_STRING_LITERAL_INVALID_UTF8: Code = Code::new("E0431").card(
        "The `\\x..` or octal escapes in this string make bytes that are not valid UTF-8 text. A \
         `string` must always be valid UTF-8. Write the character itself, or use a `\\u{...}` \
         escape.",
    );
    /// A heredoc/nowdoc's closing marker is indented with a mix of spaces and
    /// tabs — PHP 7.3's "flexible heredoc" rule (which this qualifier
    /// mirrors) requires the marker's own indentation to be one or the
    /// other, never both, since a body line's leading whitespace must match
    /// it byte-for-byte to be stripped. See
    /// `nvs_types::string_lit::heredoc_shape`.
    pub const E_HEREDOC_MIXED_INDENT: Code = Code::new("E0432").card(
        "The closing marker of this heredoc is indented with both spaces and tabs. Indent it with \
         only spaces or only tabs.",
    );
    /// A non-blank heredoc/nowdoc body line has less leading whitespace than
    /// its own closing marker — PHP 7.3's "flexible heredoc" rule requires
    /// every body line to start with at least the marker's own indentation
    /// so it can be stripped uniformly. A line that is entirely empty is
    /// exempt from this check. See `nvs_types::string_lit::dedent_heredoc_run`.
    pub const E_HEREDOC_INSUFFICIENT_INDENT: Code = Code::new("E0433").card(
        "This line of a heredoc has less indentation than the closing marker. Indent every line \
         at least as far as the closing marker, or move the marker to the left.",
    );
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
    pub const E_ARRAY_KEY_INVALID_TYPE: Code = Code::new("E0434").card(
        "An array key must be an `int`, a `uint` or a `string`. A `float`, a `bool`, `null` or an \
         enum case is not allowed as a key. Convert the key with `as`, for example `$case as int`.",
    );
    /// A `private` interface method (`rule:classes/interface-private-methods`) called from anywhere other
    /// than its own declaring interface's method bodies — it is an internal
    /// helper, never part of the interface's contract, so an implementing
    /// class (or any other interface) cannot see it at all, not even via
    /// `InterfaceName::method()`.
    pub const E_INTERFACE_PRIVATE_METHOD_NOT_VISIBLE: Code = Code::new("E0435").card(
        "This `private` method of an interface can be called only from the methods of that \
         interface. A class that implements the interface cannot call it.",
    );
    /// An `enum` case whose explicit `= expr` value is not an integer literal
    /// (or a negated one). `rule:enums/declaration` makes a case a compile-time integer
    /// constant, not a general constant-expression position.
    pub const E_ENUM_CASE_VALUE_NOT_LITERAL: Code = Code::new("E0436").card(
        "The value of an `enum` case must be a whole number written directly in the code, such \
         as `3` or `-1`. Write the number itself.",
    );
    /// An `enum` case whose value — written, or reached by `rule:enums/declaration`'s
    /// auto-increment — does not fit the enum's backing type.
    pub const E_ENUM_CASE_VALUE_OUT_OF_RANGE: Code = Code::new("E0437").card(
        "The value of this `enum` case does not fit the enum's type, `int` or `uint`. A `uint` \
         enum cannot have a negative value. If the next automatic number is too large, give this \
         case its own value.",
    );
    /// `enum Name: T` where `T` is neither `int` nor `uint` — `rule:enums/one-backing-type`
    /// gives every enum exactly one underlying *integer* type. The `string`
    /// spelling has its own, earlier diagnostic
    /// ([`E_ENUM_STRING_BACKING_UNSUPPORTED`]); this covers the rest.
    pub const E_ENUM_BACKING_NOT_INTEGER: Code = Code::new("E0438").card(
        "The type of an `enum` must be `int` or `uint`. Write `enum Name: int` or \
         `enum Name: uint`, or leave out the `: type` part.",
    );
    /// An argument passed to an `inout $x` parameter that is not a *writable place*
    /// — a bare local or a compile-time-known property. A literal, an
    /// arithmetic result or a call's own result has no storage for the callee
    /// to write back into, so the reference would have nowhere to land. An
    /// array element is refused for a different reason and permanently:
    /// copy-on-write leaves it no address that survives the call
    /// (`nvs_types::expr::args::check_inout_arg` owns why the call-site copy
    /// that would fake one is not offered).
    pub const E_INOUT_ARG_NOT_A_PLACE: Code = Code::new("E0439").card(
        "Only a variable or a property can be passed to an `inout` parameter. A value written \
         directly in the code, a calculation, an array element or a property with hooks is not allowed. Copy the value \
         into a variable, pass the variable, and then assign it back.",
    );
    /// An argument passed to an `inout $x` parameter whose type is not *exactly*
    /// the parameter's. `rule:types/declaration` leaves no room for a conversion here:
    /// the callee writes back through the reference at the declared type, so
    /// anything the caller's storage would have to be converted from on the
    /// way in would have to be converted back on the way out — silently, and
    /// lossily.
    pub const E_INOUT_ARG_TYPE_NOT_EXACT: Code = Code::new("E0440").card(
        "The variable passed to an `inout` parameter must have exactly the parameter's type. The \
         function writes a value of that type back into it. Declare the variable with the same \
         type as the parameter.",
    );
    /// A `<...>` type-argument list written after a name that takes no type
    /// parameters. `rule:types/declaration` parks user-declared generics, and two doors
    /// open in that wall — `rule:iteration/concrete-generic-implements`'s compiler-owned generic interfaces,
    /// which `nvs_hir::interfaces::RESERVED` rosters, and a `Core` member
    /// whose spec signature writes one (`Core\Json::decodeAs<T>`), which
    /// `nvs_stdlib::registry::CoreTy::Written` marks. Everything else lands
    /// here: a `type` alias (`rule:statements/nothing-gets-a-second-name` gives one no parameters of its own), a
    /// user-declared method, and a `Core` member that infers its variables
    /// from its arguments instead.
    pub const E_TYPE_ARGS_NOT_GENERIC: Code = Code::new("E0441").card(
        "This name does not take type arguments, so `<...>` is not allowed after it. Remove the \
         `<...>` part. Only some built-in types, such as `Iterator<T>`, take type arguments.",
    );
    /// A name or member written with the wrong number of type arguments,
    /// including none at all: `Iterator` on its own is as much a mistake as
    /// `Iterator<int, string>`, since the element type is the whole reason the
    /// parameter exists and `rule:types/declaration` leaves no position untyped. A call site
    /// that omits a member's required list reaches the same rule.
    pub const E_TYPE_ARG_COUNT: Code = Code::new("E0442").card(
        "This type or function needs a different number of type arguments in `<...>`. Write one \
         type for each type parameter, for example `Iterator<int>`, not `Iterator`.",
    );
    /// A `foreach` subject that is none of `rule:iteration/foreach-subjects`'s accepted
    /// shapes — an `array<T>`, an `Iterable<T>` or an `Iterator<T>`. A class
    /// reaching neither interface lands here, which is what keeps `foreach`
    /// from being another implicit-dispatch site.
    pub const E_FOREACH_SUBJECT_NOT_ITERABLE: Code = Code::new("E0443").card(
        "`foreach` can loop over an `array`, an `Iterable` or an `Iterator`, and this value is \
         none of them. Loop over an array, or make the class implement `Iterable` or `Iterator`. \
         Convert a `mixed` value first: write `$value as array<mixed>`.",
    );
    /// A `foreach ($x as $k => $v)` key binding over an `Iterable`/`Iterator`
    /// subject. `rule:iteration/two-interfaces` gives a cursor exactly `advance()` and
    /// `current()`; there is no key, and inventing a position counter would
    /// be a second thing `foreach` means.
    pub const E_FOREACH_KEY_ON_CURSOR: Code = Code::new("E0444").card(
        "A `foreach` over an `Iterable` or an `Iterator` has no keys, so `$k =>` is not allowed. \
         Write `foreach ($items as string $v)`, or loop over an array.",
    );
    /// A `yield` in a body that is not a generator's own — at file scope, or
    /// inside an anonymous function (`rule:types/anonymous-function`). `rule:iteration/generators` confines `yield` lexically to
    /// the generator's own body, which is the stated price of lowering to a
    /// state machine rather than to a coroutine.
    pub const E_YIELD_OUTSIDE_GENERATOR: Code = Code::new("E0445").card(
        "`yield` is allowed only directly in the body of a function or a method. It cannot be \
         used at the top level of a file or inside an anonymous function.",
    );
    /// A generator — a function whose body contains `yield` — declaring a
    /// return type other than `Iterator<T>`. `rule:iteration/generators`: calling one runs
    /// no user code and returns the state object, which implements exactly
    /// that interface.
    pub const E_GENERATOR_RETURN_TYPE: Code = Code::new("E0446").card(
        "A function that uses `yield` must declare its return type as `Iterator<T>`. `T` is the \
         type of the values it yields, for example `Iterator<int>`.",
    );
    /// A `return expr;` inside a generator. `rule:iteration/one-way-only` makes a generator a
    /// lazy sequence and nothing more — there is no generator return value to
    /// retrieve, so a bare `return;` (stop here) is the only form.
    pub const E_GENERATOR_RETURNS_A_VALUE: Code = Code::new("E0447").card(
        "A function that uses `yield` cannot return a value. Write `return;` to stop, or `yield` \
         the value instead.",
    );
    /// `yield from`, or a `yield` with a `key =>` half. `rule:iteration/one-way-only` rejects
    /// the first as the second spelling of an explicit re-yield loop;
    /// `rule:iteration/two-interfaces` gives `Iterator<T>` no key for the second to produce.
    pub const E_YIELD_FORM_UNSUPPORTED: Code = Code::new("E0448").card(
        "This form of `yield` is not supported. Write `yield $value;` as a statement of its own, \
         without a key and without `from`. To yield the values of another sequence, write \
         `foreach ($inner as int $v) { yield $v; }`.",
    );
    /// A concrete class that reaches an interface method nothing gives a
    /// body — a dispatch to nothing. `rule:iteration/two-interfaces`'s
    /// `Iterator<T>` is the shape that reaches it, its members being bodiless
    /// by design.
    pub const E_INTERFACE_METHOD_MISSING: Code = Code::new("E0449").card(
        "This class does not have a method that one of its interfaces requires. Add the method to \
         the class, or give the method a default body in the interface.",
    );
    /// A block-bodied `fn` anonymous function (`rule:types/anonymous-function`) with no declared
    /// return type. An expression body *is* its own answer, so it needs no
    /// annotation; a block body would need whole-body return-type inference,
    /// which `rule:types/declaration`'s "nothing is untyped, and no type ever changes by
    /// itself" does not ask the compiler to grow.
    pub const E_ANON_FN_RETURN_TYPE_REQUIRED: Code = Code::new("E0450").card(
        "An anonymous function with a `{ ... }` body must declare its return type. Write \
         `fn (int $x): int => { ... }`, or use a single expression as the body.",
    );
    /// A parameter default (`function f(int $n = ...)`) that is not a literal
    /// of the parameter's own declared type, optionally negated. Novis evaluates
    /// a default once, at signature collection, and materializes it at the
    /// call site that omitted it — so it has to be a constant this compiler
    /// can emit, not PHP's general constant *expression*. See
    /// `nvs_types::defaults`, which owns the accepted set and the shapes
    /// (`null`, an enum case) it is expected to grow next.
    pub const E_PARAM_DEFAULT_NOT_LITERAL: Code = Code::new("E0451").card(
        "The default value of a parameter must be a constant of the parameter's type. Write a \
         value such as `0`, `-1` or `'text'`, an enum case, or a `const` of another class.",
    );
    /// A parameter with no default declared *after* one that has a default.
    /// Every call supplies arguments positionally, so a required parameter
    /// behind an optional one could never be reached — PHP diagnoses the same
    /// shape.
    pub const E_PARAM_DEFAULT_ORDER: Code = Code::new("E0452").card(
        "A parameter without a default value cannot come after a parameter with one. Move it \
         before the parameters that have defaults, or give it a default too.",
    );
    /// Something other than an `rule:types/object-top` anonymous object written at a `Core`
    /// member's trailing options-bag parameter (`rule:core-api/shape-rules` R2). The bag has no
    /// runtime representation — it flattens into one argument per declared
    /// option at the call site — so it must be written out there or omitted
    /// entirely; a variable holding one cannot be passed. `rule:core-api/shape-flattens-at-the-abi`'s
    /// **shape key** parameter is refused here on the same terms and for the
    /// same reason — one flatten, one rule — and the message says "options"
    /// for either, the bag being the all-optional case of the shape.
    pub const E_OPTIONS_NOT_AN_ANON_OBJECT: Code = Code::new("E0453").card(
        "The options of this function must be written in the call itself, such as \
         `{ default: false }`. A variable that contains the options cannot be passed.",
    );
    /// A field name in an options bag that the member does not declare —
    /// usually a typo. Unlike `rule:types/shape-type`'s width subtyping, which accepts an
    /// extra field on purpose, an options bag refuses one: a misspelled option
    /// that is silently ignored is the failure `rule:core-api/shape-rules` R2 exists to prevent.
    /// A **shape key** no arm of an `rule:core-api/shape-parameter` shape parameter declares is
    /// this same code: the merged list is the whole key set either way. So is a
    /// key that belongs to an arm the anonymous object's other values did not select —
    /// `rule:core-api/shape-arms-are-disjoint`'s arm selection narrows *which* key set a call is held to, and a key
    /// outside the selected one is still not a key of this call. The two read
    /// differently and are one code on purpose: a second code would ask the
    /// reader to know which arm they were in before they could look it up.
    pub const E_UNKNOWN_OPTION: Code = Code::new("E0454").card(
        "This name is not one of the options that are allowed here. The error message lists the \
         options you can use, so check the spelling against that list. If the option was \
         removed, the message names what replaced it.",
    );
    /// `decimal ⊕ float` arithmetic, or `**` with a `decimal` base — `rule:types/arithmetic`. The same rule and the same reason as [`E_INT_UINT_ARITHMETIC`]:
    /// there is no type that represents both operands' values, so one side
    /// must be converted explicitly.
    pub const E_DECIMAL_FLOAT_ARITHMETIC: Code = Code::new("E0455").card(
        "A `decimal` and a `float` cannot be used together in arithmetic. Convert one side with \
         `as decimal` or `as float`. `**` does not accept a `decimal`, so use \
         `Core\\Decimal::pow` instead.",
    );
    /// A numeric literal placed at `decimal` whose mantissa exceeds 96 bits or
    /// whose scale exceeds 28 — `rule:types/decimal`'s layout. `Core\BigInt` is the
    /// type for a whole number beyond it, and no type holds a fraction beyond it.
    pub const E_DECIMAL_LITERAL_OUT_OF_RANGE: Code = Code::new("E0456").card(
        "This number does not fit in a `decimal`. A `decimal` has at most 28 digits after the \
         decimal point, and about 28 digits in total. Use `Core\\BigInt` for a larger whole \
         number.",
    );
    // `E0457` (`E_SINGLE_VALUE_TYPE_UNCHECKED`) is **retired**, not reused.
    // `rule:types/single-value-types`'s
    // atoms intern as real types (`nvs_types::lower::lower_atom`), so there
    // is nothing left for it to refuse.
    /// A `Core` **instance** member written as a static call —
    /// `Core\Regex\Match::text($m)` rather than `$m->text()`. `rule:core-api/shape-rules` R20
    /// gives every `Core` operation exactly one spelling, and this is the one
    /// place two could otherwise reach the same helper: an instance member's
    /// receiver is argument slot 0 at the ABI, so the static spelling would
    /// pass the arity check with the receiver written as an ordinary argument.
    pub const E_CORE_INSTANCE_MEMBER_CALLED_STATICALLY: Code = Code::new("E0458").card(
        "This `Core` method is called on an object, so it cannot be called with `::`. Write \
         `$m->text()`, not `Core\\Regex\\Match::text($m)`.",
    );
    /// A plain `->` on a receiver whose type includes `null` — `?->`, or a
    /// `!= null` test around it, is how a member of one is reached. PHP
    /// throws for this at run time; Novis refuses it while compiling, because
    /// `?T` is one union with no class to resolve a member against. Inside a
    /// block a `!= null`/`== null` test proved the receiver non-`null`
    /// (`nvs_types::locals`' narrowing) this does not fire at all — until
    /// something in that block assigns the local again, which takes the
    /// narrowing back off.
    pub const E_NULLABLE_RECEIVER: Code = Code::new("E0459").card(
        "This value can be `null`, so `->` cannot be used on it. Use `?->`, which gives `null` \
         when the value is `null`. Or check the value first with `if ($x != null) { ... }`.",
    );
    /// A `#[Json\Derive]` field that is not a same-named constructor parameter.
    /// `rule:core-classes/derive-property-list` makes a decode an ordinary `new`, so every field the codec
    /// reads has to have a parameter to arrive through; `#[Json\Field(skip:
    /// true)]` is the stated way out.
    pub const E_DERIVE_FIELD_NOT_A_PARAMETER: Code = Code::new("E0460").card(
        "Each property of a class with `#[Json\\Derive]` needs a constructor parameter with the \
         same name. Add the parameter, or add `#[Json\\Field(skip: true)]` to the property.",
    );
    /// A `#[Json\Derive]` field whose constructor parameter is declared with a
    /// different type than the property. `rule:core-classes/derive-property-list`: the two lists are one
    /// declaration for a promoted parameter, so a divergence is always written
    /// by hand and always a mistake.
    pub const E_DERIVE_FIELD_TYPE_MISMATCH: Code = Code::new("E0461").card(
        "The type of a property in a class with `#[Json\\Derive]` or `#[Db\\Derive]` must match \
         its constructor parameter. Give the property and the parameter the same type.",
    );
    /// `rule:security/derived-codec-qualifiers`'s `secret` property of a derived codec: a call
    /// that encodes a `#[Json\Derive]` class reaching one, and the declaration of one on a
    /// `#[Db\Derive]` class, whose row is read and written by the same derive.
    pub const E_DERIVE_SECRET_FIELD: Code = Code::new("E0462").card(
        "A `secret` value is never written out. A `#[Json\\Derive]` class can have a `secret` \
         property, and decoding fills it. Encoding that class does not compile. A \
         `#[Db\\Derive]` class cannot have a `secret` property. To leave the property out, add \
         `skip: true` to its `#[Json\\Field(...)]` or `#[Db\\Field(...)]` attribute.",
    );
    /// A `lateinit` property on a class carrying `#[Json\Derive]`. `rule:core-classes/derive-property-list`: `lateinit` (`rule:classes/lateinit`) is by definition not constructor-assigned,
    /// so it can never be a field.
    pub const E_DERIVE_LATEINIT_FIELD: Code = Code::new("E0463").card(
        "A class with `#[Json\\Derive]` or `#[Db\\Derive]` cannot have a `lateinit` property, \
         because every property needs a constructor parameter. Add `skip: true` to the property's \
         `#[Json\\Field(...)]` or `#[Db\\Field(...)]` attribute to leave it out.",
    );
    /// A `#[Json\Field(...)]` argument that is not one of `rule:core-classes/derive-property-list`'s
    /// options, a value that is not a literal of that option's type, or a second field attribute
    /// of one format on one property.
    pub const E_DERIVE_FIELD_ATTRIBUTE: Code = Code::new("E0464").card(
        "`#[Json\\Field(...)]` and `#[Db\\Field(...)]` accept only the options `name` and `skip`, \
         and each value must be written directly in the code. For example, write `#[Json\\Field(name: 'user_id')]` \
         or `#[Json\\Field(skip: true)]`. A property can have only one `#[Json\\Field]` and one \
         `#[Db\\Field]`. Write all its options in that one attribute.",
    );
    /// A type argument written where the member needs a *class* rather than
    /// any type — `Core\Json::decodeAs<int>`. The members that do are
    /// `nvs_stdlib::registry::WRITTEN_CLASS_MEMBERS`, and each of them reaches
    /// the written class's runtime descriptor from native code, which only a
    /// class has.
    pub const E_TYPE_ARG_NOT_A_CLASS: Code = Code::new("E0465").card(
        "This function needs a class or a shape as its type argument, not a type such as `int`. \
         Write a class such as `User`, a shape such as `{ id: uint, name: string }`, or \
         `array<User>` for a list.",
    );
    /// An `==`/`!=` — or a `switch` label, or a `match` arm — whose two static
    /// types are **disjoint**: no single value inhabits both, so the compiler
    /// already knows the answer. `rule:expressions/disjoint-comparison-refused`'s table, and
    /// `rule:expressions/switch-match-equality` for the comparison forms that are not
    /// written with the operator.
    pub const E_DISJOINT_EQUALITY: Code = Code::new("E0466").card(
        "This comparison is always false, because no value can have both types. Check that you \
         compare the right values. If a variable can be `null`, declare its type as `?T`.",
    );
    /// `+` or `+=` with an array operand. `rule:types/array-combination` removes PHP's array
    /// union operator rather than migrating it — the diagnostic names
    /// `Core\Arr::underlay`, which is what it always meant.
    pub const E_ARRAY_PLUS_UNSUPPORTED: Code = Code::new("E0467").card(
        "`+` does not combine arrays. Use `Core\\Arr::underlay($a, $b)`. It returns `$a` with \
         the keys of `$b` that `$a` does not have.",
    );
    /// `Foo::BAR` in *type* position where `Foo::BAR` is declared but is not a
    /// `string`/`int` compile-time constant — `rule:types/constant-in-type-position`. A class constant is
    /// sugar that folds to its own single-value type, so it folds only when the
    /// value has a single-value type to fold to: a `float`
    /// (`rule:types/single-value-types` has none), an
    /// `array`, an object, or an expression that is not a literal at all has
    /// none. A name nothing declares is [`E_UNKNOWN_MEMBER`] instead — that is
    /// a different mistake with a different fix.
    pub const E_SINGLE_VALUE_TYPE_NOT_CONST: Code = Code::new("E0468").card(
        "A class constant used as a type must have a `string` or `int` value. This constant has \
         another kind of value, so write its base type instead.",
    );
    /// `"z" as "a"|"b"` — `rule:types/single-value-types`: a checked conversion into a set of
    /// allowed values whose operand already names a value the set does not
    /// contain, so it would compile and then throw on every execution. The
    /// accepted set in the message is generated from the target type, never
    /// written per site. A conversion the operand does not settle — `$s as
    /// "a"|"b"` over a plain `string` — is that rule's ordinary checked row and
    /// compiles.
    pub const E_SINGLE_VALUE_TYPE_MISMATCH: Code = Code::new("E0469").card(
        "This value is not one of the values the target type allows, so the conversion always \
         fails. Write one of the values the message lists, or add this value to the target type.",
    );
    /// `Mode::Admin as Mode::Read|Mode::Write` — `rule:types/enum-case-type`'s
    /// case-subset half of [`E_SINGLE_VALUE_TYPE_MISMATCH`], on the same terms. Its
    /// own code because the set it names is a set of *cases* rather than of literal
    /// values, which is the distinction that rule exists to keep.
    pub const E_ENUM_CASE_SUBSET_MISMATCH: Code = Code::new("E0470").card(
        "This enum case is not one of the cases the target type allows, so the conversion always \
         fails. Write one of the cases the message lists, or add this case to the target type.",
    );
    /// `$obj->secret` where `secret` is declared `private` outside the class
    /// the access is written in, or `protected` outside that class and its
    /// subclasses — `rule:core-api/written-visibility`'s levels, enforced. The test is
    /// keyed on the **accessing** class and never on the receiver's static
    /// type: `$other->secret` is legal inside `Secret`'s own body and the
    /// identical line is not at file scope. A name nothing declares anywhere
    /// in the chain is [`E_UNKNOWN_MEMBER`] instead.
    pub const E_MEMBER_NOT_VISIBLE: Code = Code::new("E0471").card(
        "This property or method is `private` or `protected`, so this code cannot use it. A \
         `private` one works only inside its class, and a `protected` one also in its subclasses. \
         Make it `public`, or use a public method of the class.",
    );
    /// `public int $n = "no";` — a property's inline default is evaluated once,
    /// at signature collection, into the constant every fresh instance's slot
    /// is written with (`nvs_types::defaults`), so it has to be a compile-time
    /// constant of the property's own declared type. Its own code rather than
    /// [`E_PARAM_DEFAULT_NOT_LITERAL`] because the two accept different sets:
    /// a property may be defaulted to `[]`, to an enum case or to another
    /// class's `const` — `rule:attributes/payload-is-a-compile-time-constant`'s whole set — and a parameter to a
    /// literal only.
    pub const E_PROPERTY_DEFAULT_NOT_LITERAL: Code = Code::new("E0472").card(
        "The default value of a property must be a constant of the property's type. Write a \
         value such as `0` or `'text'`, `[]`, an enum case or a `const` of another class. Set any other value in the \
         constructor.",
    );
    /// `$obj as ?SomeClass` — `rule:expressions/nullable-conversion-availability`'s class row: `is` plus
    /// `rule:types/unions-and-mixed`'s narrowing already answers class membership, so the
    /// conversion would be R17's second spelling of a question the language
    /// already has one for. Reported for the written `?T` sugar only, since
    /// the `SomeClass|null` union spelling is not the `as ?T` form
    /// `rule:expressions/nullable-conversion` defines.
    ///
    /// **There are no exceptions**, including `Core\Uri` and `Core\Uuid`.
    /// `rule:expressions/try-parse`'s `Core\Uri::tryParse` is the member that answers a parse instead
    /// — which the help names for a class in
    /// `nvs_stdlib::registry::TRY_PARSE_CLASSES`.
    pub const E_CLASS_CONVERSION_TARGET: Code = Code::new("E0473").card(
        "`as ?ClassName` is not allowed, because a class is not a conversion target. To test the \
         class of a value, write `if ($x is ClassName) { ... }`. To read an object from text, use \
         a method of the class, such as `Core\\Uri::tryParse($s)`.",
    );
    /// `++`/`--` on a binding that is not one of `rule:types/arithmetic`'s numeric
    /// types.
    ///
    /// An increment is `± 1` and nothing else. PHP's `$s++` walking a string
    /// through `"a"`→`"b"`→`"aa"` is a divergence this takes deliberately:
    /// `rule:types/declaration` says a declared type never changes and
    /// `rule:types/arithmetic`'s table has no row that
    /// produces `"b"` from a `string` and a `1`, so there is no arithmetic
    /// here to lower. `nvs_types::expr::operators`' module doc is that
    /// decision's home.
    pub const E_INCREMENT_NOT_NUMERIC: Code = Code::new("E0474").card(
        "`++` and `--` work only on the number types `int`, `uint`, `float` and `decimal`. This \
         variable has another type. A string cannot be incremented.",
    );
    /// A `break`/`continue` whose level names no target it can jump to: a
    /// level computed at run time, a `0`, or more enclosing loops than there
    /// are — including the bare `break;` written outside every loop.
    ///
    /// PHP refuses all four at compile time too, and for the same reason:
    /// `break N` resolves to a *statically known* enclosing statement, so a
    /// level that names none has nothing to lower to.
    /// `nvs_types::locals` is where the enclosing depth is counted.
    pub const E_BREAK_LEVEL: Code = Code::new("E0475").card(
        "`break` must be inside a loop or `switch`, and `continue` must be inside a loop. A level \
         such as `break 2;` must be a fixed number from 1 to the number of loops or `switch` \
         statements around it.",
    );
    /// A `match` written with no arms at all.
    ///
    /// PHP parses one and throws `UnhandledMatchError` on every evaluation,
    /// so the construct has no reachable value there either. Novis refuses it
    /// where it is written instead: a `match` is an *expression*, and one
    /// whose every path throws has nothing for the position it sits in to
    /// bind, pass or return. Nothing that worked is lost — a written arm, or
    /// a `throw` expression, says the same thing and says it on purpose.
    pub const E_MATCH_NO_ARMS: Code = Code::new("E0476").card(
        "A `match` needs at least one arm. Add the arms you meant. If every value is an error, \
         write `default => throw new LogicError('...')`.",
    );
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
    /// (`is` proves the class, `as ClassName` converts to it), and
    /// convert or declare `mixed` for one that cannot.
    pub const E_METHOD_ON_ERASED_RECEIVER: Code = Code::new("E0477").card(
        "You can call a method only on an object of a known class. Test the class first with \
         `if ($x is ClassName) { ... }`, or convert the value with `$x as ClassName`.",
    );
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
    pub const E_ELEMENT_WRITE_THROUGH_HOOK: Code = Code::new("E0478").card(
        "You cannot change an element of a property that has hooks. Copy the property into a \
         variable, change the element, and assign the variable back to the property.",
    );
    /// A nullsafe access used as an assignment target: `$a?->b = v`.
    ///
    /// `?->` means "or `null`", and `null` is not a place: PHP refuses the
    /// same spelling with "can't use nullsafe operator in write context",
    /// because the alternative is an assignment that silently does nothing
    /// on the null path. Test the receiver instead — `if ($a !== null) {
    /// $a->b = v; }` says which of the two outcomes was meant.
    pub const E_NULLSAFE_WRITE_TARGET: Code = Code::new("E0479").card(
        "You cannot assign a value through `?->`. Test for `null` first: \
         `if ($a != null) { $a->b = $value; }`.",
    );
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
    /// (`is`, or `as ClassName`), or read the property into a typed
    /// local, write the element there, and assign it back. This is the
    /// element-write twin of [`E_METHOD_ON_ERASED_RECEIVER`].
    pub const E_ELEMENT_WRITE_THROUGH_ERASED_PROPERTY: Code = Code::new("E0480").card(
        "You cannot change an array element through a property when the class of the object is \
         not known. Convert the object first with `$x as ClassName`. You can also copy the \
         property into a typed variable, change it, and assign it back.",
    );
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
    /// it is `rule:types/absent-storage-is-never-a-zero-value`: PHP appends because the element that is
    /// not there yet reads as `""`, and no rule in Novis makes an absent
    /// element read as a zero value — which is that rule's undefined-variable
    /// case one storage kind along, not a new judgement.
    pub const E_APPEND_IN_READ_POSITION: Code = Code::new("E0481").card(
        "`$a[]` adds a new element, so it can only be the target of `=`, as in `$a[] = $value;`. \
         To read an element, write its key, such as `$a[0]`, or use `Core\\Arr::last($a)`.",
    );
    /// `$x[…]` where `$x` is not an `array<T>`, and `[…] = $x;` — `rule:types/grammar`.3's destructuring — for the same reason: every leaf is an element
    /// read, so a value with no elements has nothing to take apart.
    ///
    /// `rule:types/arrays` checks an element read and write against the array's
    /// *declared* element type, so a base that declares none — a `mixed`, a
    /// scalar, an object, or a `?array<T>` a `!== null` test has not
    /// narrowed — has no element to name and nothing to check against.
    /// PHP answers `null` with a warning for most of these, which is
    /// `rule:types/absent-storage-is-never-a-zero-value`'s family; Novis refuses at check time instead. A `string` is not
    /// an exception: `rule:types/string-is-utf8` indexes one by grapheme cluster through
    /// `Core\Str`, not through a subscript.
    pub const E_SUBSCRIPT_ON_NON_ARRAY: Code = Code::new("E0482").card(
        "`[...]` reads an element of an array, and destructuring takes an array apart. This value \
         is not an `array<T>`, so neither works. Test a `?array<T>` with `!= null` first, and \
         use `Core\\Str` for the characters of a string.",
    );
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
    pub const E_ARRAY_ELEMENT_BY_REFERENCE: Code = Code::new("E0483").card(
        "An array element cannot be a reference, so remove the `&` or the `inout`. The element is \
         a copy of the value. To share one value that can change, put it in an object.",
    );
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
    pub const E_SPREAD_SUBJECT_NOT_AN_ARRAY: Code = Code::new("E0484").card(
        "`...` inside an array such as `[...$x]` works only on an `array<T>`. Declare the value as an \
         array, or write it as an ordinary element without `...`.",
    );
    // `E0485` is retired and is never reused: it refused a `name:` argument at
    // a target whose signature carried no parameter names, and `rule:core-api/shape-rules` R2
    // leaves no such signature — a `Core` row's names are
    // `nvs_stdlib::registry::CoreMethod::names`, the synthesized `Throwable`
    // constructor's are `message` and `options`, and a reserved interface's
    // are the ones its own rule writes. A name reaching no parameter is
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
    pub const E_UNKNOWN_ARG_NAME: Code = Code::new("E0486").card(
        "The function has no parameter with this name. Check the spelling against the parameter \
         names in the message. A name cannot fill a `...$rest` parameter, so write those \
         arguments without names.",
    );
    /// One parameter given an argument twice — positionally and then by name,
    /// or by the same name twice.
    pub const E_DUPLICATE_ARG: Code = Code::new("E0487").card(
        "This parameter gets two arguments, by position and by name, or by the same name twice. \
         Remove one of them.",
    );
    /// A positional argument after a `name:` or a `...` one.
    ///
    /// PHP refuses both orderings for the same reason: which parameter a
    /// positional argument fills is its own position in the list, and neither
    /// a named argument nor an unpacked array leaves that position defined.
    pub const E_POSITIONAL_AFTER_NAMED: Code = Code::new("E0488").card(
        "A positional argument cannot come after a named argument or a `...` argument. Write \
         every positional argument first.",
    );
    /// `...$rest` at a call with no variadic parameter left for it to land in
    /// — the callee declares no `...$x` at all, or one or more of its fixed
    /// parameters is still unfilled where the spread is written.
    ///
    /// How many entries an array holds is a run-time fact, so a spread that
    /// had to fill fixed parameters would leave a call's arity uncheckable.
    /// Write the fixed arguments out and let the spread supply the tail.
    pub const E_SPREAD_ARG_NOT_VARIADIC: Code = Code::new("E0489").card(
        "`...$array` in a call can only fill a `...$rest` parameter. Write the argument for every \
         other parameter yourself, and use `...` only for the rest.",
    );
    /// `foreach (… as inout $v)` over a subject that is not a plain variable
    /// holding an `array<T>` — a call's result, a literal, a property, or an
    /// `Iterable`/`Iterator`.
    ///
    /// `inout $v` writes each element back where it came from, so there has to be
    /// a slot to write to. PHP refuses the same shapes, and a cursor is the
    /// one it names outright ("an iterator cannot be used with foreach by
    /// reference").
    pub const E_FOREACH_INOUT_SUBJECT: Code = Code::new("E0490").card(
        "`foreach` with `inout` changes each element in place, so it needs a variable that \
         contains an array. Store the value in a variable first, or remove `inout`. An \
         `Iterable` or `Iterator` cannot be used with `inout`.",
    );
    /// A `foreach (… as inout T $v)` whose `T` is not the subject's element type
    /// exactly.
    ///
    /// A by-value binding may widen — reading an `array<Dog>` as an `Animal`
    /// is sound — but a by-reference one also *writes*, and writing an
    /// `Animal` into an `array<Dog>` is not. The two directions meet only at
    /// the element type itself.
    pub const E_FOREACH_INOUT_ELEMENT_TY: Code = Code::new("E0491").card(
        "With `inout`, the type in `foreach` must be exactly the element type of the array, \
         because the loop also writes each element back. Write the element type, or remove \
         `inout` to read the elements as a wider type.",
    );
    /// A method whose body contains `yield` declaring an `inout $x` parameter.
    ///
    /// A by-reference parameter addresses a cell the *call site* stages for
    /// the duration of the call. Calling a generator runs none of its body —
    /// it allocates the state object and returns (`rule:iteration/generators`) — so that cell
    /// is gone before the first `advance()`, and there is nothing sound for
    /// the suspended frame to keep addressing.
    pub const E_GENERATOR_INOUT_PARAM: Code = Code::new("E0492").card(
        "A method that uses `yield` cannot have an `inout` parameter. Take the value as an \
         ordinary parameter, and `yield` the values you compute from it.",
    );
    /// A `fn` anonymous function declaring an `inout $x` parameter.
    ///
    /// A by-reference parameter is a contract between a call site and a
    /// declaration, and an anonymous function's type is `callable` — `rule:types/callable-is-the-only-function-type` keeps it
    /// opaque, carrying no parameter list at all, so no call site can know to
    /// stage a cell. The callable may also outlive every frame in scope where
    /// it was written.
    pub const E_ANON_FN_INOUT_PARAM: Code = Code::new("E0493").card(
        "An anonymous function written with `fn` cannot have an `inout` parameter. Take the value as an \
         ordinary parameter and `return` the result, or pass an object and change its properties.",
    );
    /// An `rule:types/anonymous-object` anonymous object writing one field name twice —
    /// `{ a: 1, a: 2 }`.
    ///
    /// A shape's fields are a set: the type `{ a: int }` names one slot `a`,
    /// and there is no layout under which an anonymous object's two `a`s are both
    /// reachable. PHP's nearest neighbour is a duplicate *array* key, where
    /// the last write wins silently, but an array is a map and a shape is a
    /// record — and the two sides here would not even agree on which write
    /// survives: the interned shape reads the first of the pair while a
    /// class carries one slot per name. Refusing where it is written is the
    /// only answer that keeps both readings out of the language.
    pub const E_DUPLICATE_SHAPE_FIELD: Code = Code::new("E0494").card(
        "This anonymous object writes the same field name twice, as in `{ a: 1, a: 2 }`. Each field \
         can appear only once, so remove one of them.",
    );
    /// `->` on a receiver whose declared type can never hold an object —
    /// `int $i = 5; echo $i->name;` — or on one that names no single class,
    /// such as a union of two.
    ///
    /// PHP warns and yields `null` here; `rule:types/a-declared-type-answers-before-the-program-runs` makes it a
    /// check-time error instead, for row 8's reason: a declared type is what
    /// makes the answer knowable before the program runs, and nothing in Novis
    /// makes an absent thing read as a zero value. `mixed` is the one receiver
    /// that keeps PHP's *timing* — `rule:types/conversion`'s one unchecked position, so
    /// it defers to `rule:types/erased-member-access`'s name-keyed fetch and its catchable throw.
    pub const E_RECEIVER_HAS_NO_PROPERTIES: Code = Code::new("E0495").card(
        "`->` reads a property of an object, and the type of this value is not one class. \
         Convert the value to the class you expect with `$x as Box`, or declare it `mixed` to \
         check it only when the program runs.",
    );
    /// A class named through a value the checker can resolve to no class at
    /// all, at any of `rule:types/class-reference-sites`' three sites:
    /// `new $name()`, `$name::f()` and `$x is $name`.
    ///
    /// It is `rule:types/conversion`'s rule: a class name is written, never
    /// computed, which is the same line `$$var` and `eval` are already on. The
    /// three spellings share one report
    /// (`nvs_types::expr::members::reject_dynamic_class_name`) because they are
    /// one mistake, and a second code for the same rule at one of them would be
    /// a distinction the language does not make. A `class<T>` operand is not
    /// this mistake at any of the three — that is the checked dynamic form the
    /// rule gives them — so the help names the conversion an author can take:
    /// `$name as class<Base>`.
    ///
    /// A written name that resolves to *nothing* is not here: that is the
    /// ordinary `E0303`, exactly as `new Undeclared()` already reports it.
    pub const E_DYNAMIC_CLASS_NAME: Code = Code::new("E0496").card(
        "A class name must be written in the code, so `new $name()`, `$name::f()` and \
         `$x is $name` need a class. Write the class name, or convert the value once with \
         `$name as class<Base>` and use the result.",
    );
    // `E0497` (`E_INSTANCEOF_SUBJECT_NOT_OBJECT`) is **retired**, not reused.
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
    pub const E_ISSET_NOT_A_VARIABLE: Code = Code::new("E0498").card(
        "`isset` works only on a variable, an array element or a property. For any other value, \
         such as the result of a call, write `$value != null` instead.",
    );
    /// `static::$prop` — late static binding on a *static property*, whose
    /// storage Novis resolves at compile time.
    ///
    /// The spelling reads as a lookup against the **called** class, so a
    /// reader expects a subclass that redeclares the property to get its own
    /// storage through it. Novis's slot is fixed where the access is written
    /// (`nvs_ir::ir::StaticProp`), so for such a subclass the spelling would
    /// silently name the parent's storage instead, and it is refused.
    /// `self::$prop` and a written class name both say exactly which storage
    /// is meant and are unaffected (`rule:statements/static-is-a-member-modifier`).
    ///
    /// **The last code in the E04xx band**, which is full. The types band
    /// continues at `E07xx`, and `E0500` is never issued, its own digits
    /// reading as the IR-and-codegen band; this module's legend table and the
    /// paragraph under it say why.
    pub const E_STATIC_PROPERTY_LATE_BOUND: Code = Code::new("E0499").card(
        "`static::$prop` is not allowed for a static property. Write `self::$prop` for the class \
         this code is in, or write the name of the class whose property you mean.",
    );
}
