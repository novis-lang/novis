//! The type checker's codes, continued from `types.rs`: `E07xx`.
//!
//! The range is full at `E0799`, so a new code of this area goes in
//! `types_e08.rs`.

codes! {
    range = "E07";

    //
    // The E04xx band filled at `E0499`. Max-plus-one yields `E0500`, whose
    // band digits read as E05xx — IR and codegen — so that number is never
    // issued and the types band continues here instead. The paragraph under
    // this module's legend table owns the reasoning.
    /// An array element written through a root that is not a **place**:
    /// `$h->rows()["a"] = "y"`, `[1, 2]["0"] = "z"`, `($c ? $a : $b)["k"] = v`.
    ///
    /// `rule:types/arrays`'s copy-on-write separation has to be written back into
    /// whatever holds the array, and a temporary holds it nowhere — the write
    /// would land in a value dropped at the end of the statement. PHP 8.5
    /// accepts the spelling and discards the write with no diagnostic at all
    /// (checked with `php -r`, not assumed), which makes this a deliberate
    /// divergence (`rule:types/an-element-write-needs-storage-to-write-back-into`)
    /// rather than a PHP-compatible refusal like `E0478` beside it.
    ///
    /// Parentheses are **not** a temporary: `($a)["0"] = "y"` writes `$a["0"]`
    /// here exactly as it does in PHP, because
    /// `nvs_syntax::ast::Expr::unparenthesized` is what finds the root.
    pub const E_ELEMENT_WRITE_ROOT_NOT_A_PLACE: Code = Code::new("E0700").card(
        "An array element can only be written through a variable or a property. Here the array \
         comes from a call or another value that is not stored anywhere, so the write would be \
         lost. Store the array in a variable, write the element there, and assign it back.",
    );
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
    pub const E_ASSIGN_BY_REFERENCE: Code = Code::new("E0701").card(
        "Novis has no references, so `$a = &$b` is not allowed. Remove the `&` to assign a copy. \
         If a function must change a variable, declare that parameter `inout`.",
    );
    /// An argument binding an `inout` parameter, written without the marker.
    ///
    /// `rule:statements/inout-is-written-at-the-call` writes the word at both ends, and this is the half a
    /// rename alone would not have bought: `Adder::bump($n)` is otherwise
    /// indistinguishable at the point of call from `Adder::sum($a, $b)`,
    /// and only one of them writes to its caller's storage. The marker is
    /// deliberately not inference-assisted — a marker the compiler supplies
    /// is not a marker — so the omission is an error rather than a lint.
    pub const E_INOUT_ARG_MISSING: Code = Code::new("E0713").card(
        "This argument fills an `inout` parameter, so the call must say so. Write `inout` before \
         the argument, as in `bump(inout $n)`. The function writes a new value back into it.",
    );
    /// `inout` written at an argument that binds a by-value parameter, or at
    /// one that binds nothing a signature can name — a spread's entries, or
    /// any argument of a call through a `callable`.
    ///
    /// The mirror of [`E_INOUT_ARG_MISSING`]: a marker that is allowed to be
    /// wrong is worth nothing to the reader, so `rule:statements/inout-is-written-at-the-call` makes the extra
    /// one an error too. Through a `callable` it can never be right —
    /// `rule:types/callable-is-the-only-function-type` keeps that type opaque and [`E_ANON_FN_INOUT_PARAM`] refuses
    /// the declaration end outright — and a spread hands over a subject's
    /// entries rather than the subject, which is the same reason
    /// [`E_INOUT_ARG_NOT_A_PLACE`] wants one storage location. Through a
    /// **`mixed` receiver** it can never be right either, and for the reason
    /// the runtime dispatch refuses such a callee outright: an `inout`
    /// parameter list is packed and written back at the *call site*, which is
    /// the one thing a call whose callee is unknown until it runs cannot do.
    pub const E_INOUT_ARG_UNEXPECTED: Code = Code::new("E0714").card(
        "`inout` is written before an argument whose parameter is not `inout`. Remove `inout`, or \
         declare the parameter `inout`. A `...` argument and a call through a `callable` or a \
         `mixed` value never take `inout`.",
    );
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
    ///   lifts it is the one `->` already requires, and `Core\Reflect`
    ///   (`rule:core-classes/reflect`) is the door for a receiver whose type was genuinely erased.
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
    pub const E_CLASS_NAME_CONST_NOT_STATIC: Code = Code::new("E0702").card(
        "`::class` needs a class name or a value that is always an object. For a `mixed` or \
         nullable value, test it first with `if ($v is Foo)`. For a `class<T>` value, write \
         `$c as string`.",
    );
    // `E0703` is retired and is never reused: `spawn script` lowers through
    // `nvs-ir`, so there is nothing left for it to refuse.
    // `E0704` is retired and is never reused: `require` used for its
    // **value** is `rule:statements/a-require-expression-is-mixed`'s `mixed` and lowers, the site calling the
    // target file's own script frame and keeping what it hands back
    // (`nvs_ir::lower::Lowering::lower_expr`).
    /// `-`, `+` or `~` over an operand `rule:types/arithmetic`'s arithmetic table has no
    /// row for — a `string`, a `bytes`, an `array<T>`, a `bool`, `null` or a
    /// `callable`. An enum value takes [`E_ENUM_ARITHMETIC_UNSUPPORTED`]
    /// instead, as it does under every other operator. The sibling of [`E_INCREMENT_NOT_NUMERIC`]
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
    pub const E_UNARY_ARITH_NOT_NUMERIC: Code = Code::new("E0705").card(
        "Unary `-`, `+` and `~` work only on numbers. Novis does not convert the operand to a \
         number by itself. Convert it first, as in `-($x as int)`.",
    );
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
    pub const E_BITWISE_NOT_INTEGER: Code = Code::new("E0706").card(
        "The bitwise operators `&`, `|`, `^`, `~`, `<<` and `>>` work only on `int` and `uint`. \
         Convert the operand to a whole number first, as in `($x as int) & 1`.",
    );
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
    pub const E_NO_STRING_FORM: Code = Code::new("E0707").card(
        "This value has no text form, so it cannot be joined with `.`, put in a string or printed. \
         Convert `bytes` or a class reference with `as string`, an enum case with `$case as int`, \
         and an array with `Core\\Json::encode($a)`. A function has no text: call it and use what \
         it returns.",
    );
    /// An `expr as T` whose operand and target name no row of `rule:types/conversion`'s
    /// conversion table, its `string` ↔ `bytes` pair, its `decimal` rows and its enum
    /// rows included.
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
    pub const E_NO_CONVERSION: Code = Code::new("E0708").card(
        "`as` cannot convert this type to that type. Novis has a fixed list of conversions, and \
         this pair is not on it. The help under the error shows the way to write what you meant.",
    );
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
    pub const E_NULLABLE_CONVERSION_CANNOT_FAIL: Code = Code::new("E0709").card(
        "This `as ?T` conversion can never fail, so its result is never `null`. Write `as T` \
         instead.",
    );
    /// A `Core`-owned class rendered as text where the registry gives it no
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
    pub const E_CORE_CLASS_NOT_STRINGABLE: Code = Code::new("E0710").card(
        "This `Core` class has no text form, so it cannot be printed, joined with `.` or converted \
         with `as string`. Call one of its methods that returns the text you want.",
    );
    /// An `expr as T` whose target the conversion cannot check the value
    /// against. Two kinds of target take it.
    ///
    /// **An object target nothing can be tested against**: plain `object`,
    /// `callable`, a `Core` namespace class, or a shape whose fields carry a
    /// `tainted` or `secret` qualifier, from an operand that is not already an
    /// object. `rule:types/conversion` tabulates no row producing an object,
    /// and a class or a shape target is admitted only because it can be
    /// *checked*: the conversion out of `mixed` tests the value's runtime
    /// class, or walks a shape's fields as `is` does, and throws when it
    /// misses. `object` and `callable` name no class, and nothing is ever an
    /// instance of a `Core` namespace class, so there is no descriptor to test
    /// against. A qualifier is erased before codegen, so a shape carrying one
    /// has a field no walk can check. An operand that is already an object is
    /// the free widening row: `$plain as object` runs nothing, because both
    /// sides are one pointer.
    ///
    /// **An `array<U>` whose element type no tag decides**: a class, a shape,
    /// `callable`, `object`, an enum, a union (`?T` included) or a
    /// single-value type, at any nesting depth. The `array<T> as array<U>` row
    /// checks each element by its tag alone, and a tag proves objecthood but
    /// no class, names an enum's backing but no enum, and admits values a
    /// union or a single-value type does not. `nvs_types`'
    /// `reject_uncheckable_element_type` is where the list is decided, and its
    /// help names `array<mixed>` and a per-element `as` as the fix.
    ///
    /// Each time the conversion could only *assert* what it cannot verify, so
    /// the answer is a diagnostic where it is written. The sibling for a target
    /// that *can* be tested is [`E_NO_CONVERSION`], which refuses the pair
    /// sharing no value at all; the two never fire together, because a target
    /// can be tested or it cannot.
    pub const E_UNTESTABLE_CONVERSION_TARGET: Code = Code::new("E0711").card(
        "`as` can only convert a value to a type it can check the value against. `object`, \
         `callable` and a `Core` namespace class have nothing to check. Convert to a class, as in \
         `$v as User`, or to a shape, as in `$v as { id: int }`.\n\n\
         `as array<T>` checks each element by the kind of value it is. That works when `T` is \
         `mixed` or a simple type such as `int` or `string`. It does not work for a class, a \
         shape, an enum, or a union such as `?int` or `'on'|'off'`. Convert to `array<mixed>`, \
         then convert each element where you read it, as in `$item as User`.",
    );

    /// A `name:` argument at a call through a `callable`.
    ///
    /// `rule:types/anonymous-function` gives `callable` no parameter list — it is one opaque type
    /// whatever callable a variable holds — so there is no parameter for a name
    /// to fill, at the site or below it: a callable value records its arity and
    /// its parameter *tags*, never their names, so nothing at run time could
    /// resolve one either. PHP can only allow it because a `Closure` there
    /// carries its declaration.
    ///
    /// The same rule as [`E_ANON_FN_INOUT_PARAM`], read from the call site's
    /// end rather than the anonymous function's. It has no sibling at a *resolved*
    /// target: every signature the checker builds names its parameters
    /// (`rule:core-api/parameters-are-callable-by-name`), so a `name:` there is a spelling question and never an
    /// absence — [`E_UNKNOWN_ARG_NAME`] is the whole of it. A `...`
    /// argument is not refused here: how many arguments it hands over is its
    /// own run-time length, which needs no parameter list to be meaningful.
    ///
    /// A call through a **`mixed` receiver** takes the same code, because it
    /// is the same absence: `rule:types/erased-member-access` defers that call to the receiver's
    /// runtime class, whose method row carries the callee's arity and
    /// parameter tags and — for a callable value's reason — never its parameter
    /// names.
    pub const E_NAMED_ARG_THROUGH_CALLABLE: Code = Code::new("E0712").card(
        "A call through a `callable` or a `mixed` value cannot use named arguments. The compiler \
         does not know the parameter names there. Pass the arguments by position.",
    );

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
    pub const E_ORDERING_HAS_NO_ROW: Code = Code::new("E0715").card(
        "`<`, `<=`, `>`, `>=` and `<=>` compare numbers, and objects of a class that implements \
         `Comparable`. They also compare two `bool` values, but not a `bool` and a number. They do \
         not compare strings, arrays, enum cases or `null`. Use `Core\\Str::compare` for text, and \
         `as int` for an enum case.",
    );

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
    ///
    /// A union that can hold `null` takes this code too, under the arithmetic
    /// operators, and [`E_BITWISE_NOT_INTEGER`] under the bitwise ones: its
    /// tag would throw on the `null` arm, so it is refused where it is written.
    pub const E_ARITHMETIC_HAS_NO_ROW: Code = Code::new("E0716").card(
        "The arithmetic operators work only on `int`, `uint`, `float` and `decimal`. Novis does \
         not convert the operand to a number by itself. Convert it with `as` first. A value \
         that can be `null`, such as a `?float`, is not allowed either. Test it with \
         `if ($x != null)` first, or give it a default with `($x ?? 0)`.",
    );

    /// `%` with a `float` operand.
    ///
    /// The one row this band refuses that both operands *are* numbers for.
    /// `rule:types/arithmetic`'s "either operand a `float`" row is written for the
    /// arithmetic operators as a family, but `%` is the one member of it PHP
    /// does not answer that way: PHP converts both operands to an integer and
    /// returns an integer, where the row would return a `float`. The two are
    /// different answers for `7.5 % 2`, no rule settles which of them Novis
    /// gives, and `as` is one character away — so the operator is refused at
    /// **both** ends rather than guessed at either. The other end is the
    /// tagged one, `nvs_runtime::helpers::value_arith`, where the same rule
    /// arrives as a catchable throw because only the runtime tags can see it.
    ///
    /// `decimal % float` is [`E_DECIMAL_FLOAT_ARITHMETIC`] instead: that pair
    /// has no common arithmetic type at all, which is the earlier objection.
    pub const E_FLOAT_MODULO: Code = Code::new("E0717").card(
        "`%` does not work on a `float`. For a whole-number remainder, write \
         `($a as int) % ($b as int)`. For a floating-point remainder, call `Core\\Math::mod($a, $b)`.",
    );
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
    pub const E_VOID_IS_NOT_AN_OPERAND: Code = Code::new("E0718").card(
        "This call returns `void`, so it has no value to use with an operator. Give the function a \
         return type and `return` a value, or call it as its own statement.",
    );
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
    pub const E_VOID_IS_NOT_A_CONDITION: Code = Code::new("E0719").card(
        "This call returns `void`, so it has no value to test in a condition. Give the function a \
         return type and `return` a value, or call it as its own statement.",
    );
    /// An `implements I by $field;` clause whose `$field` cannot answer `I` —
    /// `rule:classes/delegation-by-property` bullet 1.
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
    pub const E_DELEGATE_TYPE_MISMATCH: Code = Code::new("E0720").card(
        "In `implements I by $property`, `$property` must be a property of this class. Its type \
         must be a class or interface that implements `I`, and it cannot be nullable.",
    );
    /// A member of a delegated interface whose shape `rule:classes/delegation-by-property`'s
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
    /// write the member on the class by hand, which
    /// `rule:classes/delegation-by-property` already allows and
    /// which the forward would have lost to.
    pub const E_DELEGATE_MEMBER_NOT_FORWARDABLE: Code = Code::new("E0721").card(
        "`implements I by $property` cannot forward a `static` method, or a method with a `...` \
         or `inout` parameter. Write that method on the class yourself.",
    );
    /// A visibility keyword on a parameter of a method that is not the
    /// `constructor` — `rule:classes/delegation-by-property`'s own backlog line.
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
    pub const E_PROMOTED_PARAM_OUTSIDE_CONSTRUCTOR: Code = Code::new("E0722").card(
        "Only a `constructor` parameter can have `public`, `protected` or `private`, because the \
         keyword declares a property. Remove the keyword here. To declare a property, write it on \
         the class.",
    );
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
    pub const E_FOREACH_KEY_TY: Code = Code::new("E0723").card(
        "Array keys are always strings, so the key variable of a `foreach` must be `string`. \
         Write `foreach ($a as string $k => $v)`, and convert `$k` inside the loop if you need \
         another type.",
    );

    /// `rule:security/secret-sinks-refuse`'s debug-dump sink, as
    /// `rule:errors/record-transformations`'s redaction row states it: a `secret`-qualified value written at a
    /// `Core\Debug::dump`/`render` call site is refused where it is written.
    ///
    /// It is a *call-site* rule rather than a parameter type, for
    /// `rule:security/secret-sinks-refuse`'s reason and `Core\Log::write`'s: both members declare
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
    pub const E_SECRET_DEBUG_ARGUMENT: Code = Code::new("E0724").card(
        "A `secret` value cannot be passed to `Core\\Debug::dump` or `Core\\Debug::render`, \
         because a person reads that output. If you really need to show it, call \
         `Core\\Secret::reveal($value, \"reason\")` first.",
    );
    /// An attribute payload's field value is not a compile-time constant —
    /// `rule:attributes/payload-is-a-compile-time-constant`. The whole payload object is resolved once, at compile time, into the
    /// unit's constant pool, the same storage class an enum case's backing
    /// value already uses; there is no "evaluate this attribute's arguments"
    /// step at class-definition time for a variable, a call or a `new` to be
    /// evaluated *in*, which is what PHP's lazily-constructed attribute
    /// object has and this one deliberately does not.
    pub const E_ATTRIBUTE_VALUE_NOT_CONSTANT: Code = Code::new("E0725").card(
        "An attribute value must be known when the program compiles. Write a number, a string \
         in quotes, a class constant or an enum case. A variable, a function call or `new` is not allowed.",
    );
    /// The named form of an attribute names something that is not a
    /// shape-typed `type` alias —
    /// `rule:attributes/attach-sites-and-forms`. `Name` there is never a class and never a new namespace of
    /// attribute kinds: it is a pre-existing alias whose right-hand side is a
    /// shape, and its whole job is to be the type the attribute's payload
    /// object is checked against.
    ///
    /// A name nothing declared at all takes [`E_UNDEFINED_CLASS`] instead,
    /// exactly as any other unresolvable name does — an attribute name is not
    /// a second namespace, so it gets no "no such attribute" of its own. This
    /// code is for a name that *does* denote something and denotes the wrong
    /// thing: a class (the spelling PHP's attributes would have instantiated),
    /// an interface, an enum, or a `type` alias for something that is not a
    /// shape.
    pub const E_ATTRIBUTE_NAME_NOT_A_SHAPE: Code = Code::new("E0726").card(
        "The name of an attribute must be a `type` alias for a shape. This name is a class, an \
         interface, an enum or another kind of type. Declare a shape `type` and use its name.",
    );
    /// A `secret`-qualified class constant reaches an attribute payload —
    /// `rule:security/secret-sinks-refuse`
    /// 's attribute-payload sink, the one that exists *because* of
    /// `rule:attributes/payload-is-a-compile-time-constant`: a payload holds only compile-time constants, and a class constant
    /// is one of them, so the qualifier's own storage class is the only way a
    /// `secret` value could get in there at all.
    ///
    /// It is a sink rather than a mismatch because of where the value ends up:
    /// the payload is folded into the compiled unit's constant pool and handed
    /// back by `rule:attributes/structural-retrieval` to anything that asks, so the credential is
    /// written into the program's own metadata rather than read at run time
    /// from somewhere a capability guards.
    ///
    /// A separate code from [`E_SECRET_DEBUG_ARGUMENT`] because the way out is
    /// a different one. That sink has `Core\Secret::reveal()` in front of it;
    /// this one has nothing, `reveal` being a call and a payload admitting
    /// none — so the help names the fix that exists, which is to keep the
    /// secret out of the metadata entirely.
    pub const E_SECRET_ATTRIBUTE_PAYLOAD: Code = Code::new("E0727").card(
        "A `secret` class constant cannot be used in an attribute. Attribute values are stored in \
         the compiled program, where any code can read them. Keep the secret out of the attribute.",
    );
    /// A `Core\Attributes::get<T>` whose target carries more than one attached
    /// payload object satisfying `T` — `rule:attributes/retrieval-folds-while-checking`.
    ///
    /// A declaration's attached-attribute list is fully static, so "which one
    /// did I get?" is a question this compiler answers rather than one a test
    /// run answers. `::all<T>` is the member that wants every match, and the
    /// help names it. `Core\Program::implementingWith` and
    /// `Core\Router\Match::attribute` report it for the same mistake on one
    /// listed class or one routed method.
    pub const E_ATTRIBUTE_RETRIEVAL_AMBIGUOUS: Code = Code::new("E0728").card(
        "More than one attribute on this declaration matches the shape, so `get` cannot choose \
         one. Make the shape more specific, or call `Core\\Attributes::all` to get every match.",
    );
    /// The `<T>` written at a `Core\Attributes::get`/`all` call site is not a
    /// shape type — `rule:attributes/structural-retrieval`.
    ///
    /// Retrieval is *structural*: `T` is what an attribute's payload object is matched
    /// against under `rule:types/shape-type`'s width subtyping, so a `T` that is not a
    /// shape names nothing an attribute payload could ever satisfy.
    /// `Core\Program::implementingWith` and `Core\Router\Match::attribute`
    /// report it for their shape argument too.
    pub const E_ATTRIBUTE_TYPE_ARG_NOT_A_SHAPE: Code = Code::new("E0729").card(
        "The type argument of an attribute lookup must be a shape. Write an inline shape `{...}`, \
         or the name of a `type` alias for one.",
    );
    /// The `$target` of a `Core\Attributes::get`/`all` call does not name a
    /// declaration this unit holds — `rule:attributes/structural-retrieval`.
    ///
    /// That rule fixes the spelling: a method (a constructor included) is named by
    /// its own method reference `Foo::bar`, and a class by
    /// its `constructor`'s. Retrieval is resolved entirely at compile time, so
    /// the reference is *inspected* where it is written rather than evaluated
    /// — anything else has no declaration to read an attribute list off.
    pub const E_ATTRIBUTE_TARGET_NOT_A_DECLARATION: Code = Code::new("E0730").card(
        "The target of `Core\\Attributes::get` or `all` must name a declaration. Write \
         `Foo::bar` for a method, and `Foo::constructor` for the class itself.",
    );
    /// An attached payload object satisfies the `T` a `Core\Attributes` retrieval
    /// asked for, but holds a value this compiler cannot materialize.
    ///
    /// `rule:attributes/retrieval-folds-while-checking` replaces the call with the payload itself, so every value
    /// in a matched payload has to have a constant form. Every spelling
    /// `rule:attributes/payload-is-a-compile-time-constant` admits has one — the payload is folded under the scope it was
    /// *written* in, so a class constant, `Foo::class` and an enum case each
    /// resolve to the value a read of the same name inlines
    /// (`nvs_types::retrieval`'s own docs own that). What is left is the
    /// residue: a class constant whose *own* declaration folded to no value,
    /// which is [`E_CLASS_CONST_NO_CONSTANT_FORM`]'s gap surfacing at the one
    /// site that needs the value rather than the name.
    pub const E_ATTRIBUTE_PAYLOAD_UNFOLDABLE: Code = Code::new("E0731").card(
        "This attribute uses a class constant whose value the compiler cannot compute. Write the \
         value in the attribute directly, or give that constant a value written directly in the \
         code.",
    );
    /// The method reference `$m->method(...)` written on a
    /// `mixed` receiver.
    ///
    /// Every other erased receiver takes [`E_METHOD_ON_ERASED_RECEIVER`] for
    /// the whole call; `mixed` is `rule:types/conversion`'s one unchecked position and
    /// defers instead, so a *call* through it dispatches on the receiver's
    /// runtime class. This spelling does not call at all: `rule:types/callable-values` makes it a
    /// callable **value**, which carries the callee's arity and parameter tags
    /// in the value itself (`nvs_runtime::callable`), and there is no class
    /// here to read either off — the receiver's descriptor answers a call it
    /// is present at, not a value that outlives the site. Narrowing the
    /// receiver, or calling the member directly, is the fix; both are what the
    /// help names.
    pub const E_METHOD_REF_ERASED_RECEIVER: Code = Code::new("E0732").card(
        "A method reference like `$value->method(...)` needs a value of a known class, and this \
         value is `mixed`. Narrow the value first with `is` or `as ClassName`, or call the method \
         directly.",
    );
    /// A `#[Test]` method whose declaration is not the shape `rule:testing/test-attribute`
    /// requires: `static`, not `public`, returning anything but `void`, or
    /// declared with no body.
    ///
    /// One code for the family rather than four, because it is one question —
    /// what shape a test method must have — and the runner asks it once: it
    /// constructs the class and calls the member with no arguments and no
    /// result, so a `static` member has no receiver for a `#[Fixture]` to be
    /// installed on, a non-`public` one cannot be called from outside the
    /// class at all, a returned value has nowhere to go and nothing that
    /// would look at it, and an `abstract` one has no code to call. Each help
    /// names the modifier or the annotation to change.
    ///
    /// The fifth error in that bullet — two `#[Test]` methods with one name —
    /// is [`E_DUPLICATE_DECLARATION`] instead, being the same mistake the
    /// option written twice already draws, and the sixth (`skip: true`) is the
    /// option roster's own type check.
    pub const E_TEST_METHOD_SHAPE: Code = Code::new("E0733").card(
        "A `#[Test]` method must be `public`, must not be `static` or `abstract`, and must return \
         `void`. Write it as `public function name(): void { ... }`.",
    );
    /// A `#[Test(retries: n)]` with no `because:` beside it.
    ///
    /// `rule:testing/runner-is-strict` grants retries and charges a written reason for them in
    /// the same sentence: a retry that nobody had to justify is how a suite
    /// stops noticing that it is unreliable, and the reason is what a reader
    /// of the attribute has instead of the run that produced it. It is a
    /// separate code from the option roster's own type check because nothing
    /// about `retries: 2` is ill-typed — the mistake is the option that is
    /// *absent*, which is the one thing an all-optional bag cannot say.
    pub const E_TEST_RETRIES_WITHOUT_REASON: Code = Code::new("E0734").card(
        "A `#[Test]` with `retries:` must also give `because:`, a sentence that says why the test \
         may fail. Write `#[Test(retries: 2, because: \"the server is slow to start\")]`.",
    );
    /// A `#[Fixture]` method the runner could not build a value from.
    ///
    /// `rule:testing/fixtures` builds a fixture **once, in the parent isolate**, and
    /// injects the value it returns into each test that declares a parameter
    /// of its type — so a fixture that is not `static` has no instance to be
    /// built against (each test constructs its own, which is the opposite of
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
    /// [`E_DUPLICATE_DECLARATION`]: `rule:testing/fixtures` resolves by type, so it is one
    /// declaration made twice.
    pub const E_FIXTURE_METHOD_SHAPE: Code = Code::new("E0735").card(
        "A `#[Fixture]` method must be `public` and `static`, and must return a value. A method \
         cannot be a `#[Test]` and a `#[Fixture]` at the same time.",
    );
    /// A `#[Test]` or `#[Fixture]` parameter whose type no `#[Fixture]` of the
    /// class supplies.
    ///
    /// `rule:testing/fixtures` resolves a parameter **by type**, while compiling, so that
    /// "an unsatisfiable parameter is a diagnostic rather than a null at
    /// runtime" — this is that diagnostic, and it is the `Widget` line of that
    /// section's own worked example. One code covers a test's parameter and a
    /// fixture's own, because it is one question asked of one roster: what
    /// supplies this type. `rule:testing/data-rows`'s rows are the other answer, and they
    /// fill a parameter **by name**, so this fires only where neither roster
    /// reaches it — which is why the help names both.
    pub const E_FIXTURE_PARAMETER_UNSUPPLIED: Code = Code::new("E0736").card(
        "No `#[Fixture]` of this class returns this parameter's type, and no `#[TestWith]` row \
         gives it a value. Add a fixture that returns this type, or a row with this parameter's \
         name.",
    );
    /// A `#[Fixture]` that requires itself, directly or through others.
    ///
    /// `rule:testing/fixtures`'s last sentence: a fixture may declare fixture parameters
    /// of its own, and a cycle among them is a compile error. It is separate
    /// from [`E_FIXTURE_PARAMETER_UNSUPPLIED`] because every parameter on such
    /// a cycle *is* supplied — by a roster that cannot be built in any order,
    /// which is a fact about the chain rather than about any one parameter.
    pub const E_FIXTURE_CYCLE: Code = Code::new("E0737").card(
        "These fixtures need each other in a circle, so none of them can be built first. Move the \
         value they share into a fixture of its own to break the circle.",
    );
    /// A `#[TestWith(...)]` data row that does not describe the method it is
    /// attached to.
    ///
    /// `rule:testing/data-rows` matches a row's anonymous object against the method's
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
    pub const E_TEST_ROW_FIELD: Code = Code::new("E0738").card(
        "A `#[TestWith]` row must give a value for each parameter of its `#[Test]` method, by the \
         parameter's name and of its type. Every row of one method gives the same parameters.",
    );

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
    pub const E_MISSING_RETURN: Code = Code::new("E0739").card(
        "This method declares a return type, but one path reaches the end of the body without a \
         `return` or a `throw`. Add a `return` with a value or a `throw` at the end of that path.",
    );
    /// A method reference written on `new`: `new C(...)`.
    ///
    /// `rule:types/callable-values`'s kept list is a list of *members* — `Class::method`,
    /// `$obj->method(...)`, `self`/`static`/`parent::method` — and a
    /// constructor is not one of them: `new` names a class, and the callable
    /// this syntax builds carries a callee, not an allocation. PHP refuses it
    /// for the same reason ("cannot create Closure for new expression"), so
    /// refusing it is the PHP-compatible answer as well as the only one with a
    /// meaning. `fn (): C => new C(…)` is the anonymous function that was wanted, and it
    /// is what the help names.
    ///
    /// Separate from [`E_METHOD_REF_ERASED_RECEIVER`] because the
    /// two are opposite failures: that one resolves a member and has no class
    /// to bind it to, this one names a class and has no member.
    pub const E_METHOD_REF_NEW: Code = Code::new("E0740").card(
        "`new Foo(...)` cannot be used as a method reference, because a constructor is not a \
         method. Write an anonymous function instead: `fn (): Foo => new Foo()`.",
    );

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
    pub const E_STATIC_RETURN_NOT_CALLED_CLASS: Code = Code::new("E0741").card(
        "A method with the return type `static` must return `$this`, `new static(...)`, or the \
         result of another method that returns `static`. If the method always returns this exact \
         class, declare the return type `self`.",
    );

    /// `void` or `never` stands somewhere that is not a return type: a
    /// parameter, or the type a binding holds.
    ///
    /// `rule:types/grammar` says both are return-only, and there is nothing else they
    /// could mean anywhere else: `void` is the absence of a value and `never`
    /// is the empty type, so no argument satisfies either and no name can hold
    /// one. A method declaring one as a parameter has no callable arity — every
    /// call site would be refused for a reason phrased about the argument
    /// rather than about the declaration that made it impossible.
    ///
    /// Refused at the declaration for that reason. A parameter is caught in the
    /// one pass that reads every method signature (`nvs_types::signatures`), so
    /// an abstract method and an interface signature are covered as well as a
    /// body; a binding is caught in `nvs_types::locals`, which is also where a
    /// `var` taking its type from a call that hands nothing back is refused.
    /// Unrefused, every one of these type-checks and reaches the back end:
    /// `never` panics `nvs_ir::lower`'s representation map, `void` lowers and
    /// dies in `nvs-codegen` reading a value of representation `void`, and a
    /// binding defined by a `void` call leaves its name standing for nothing,
    /// which the IR reports as an operand used before it is defined.
    pub const E_VOID_OR_NEVER_OUTSIDE_RETURN: Code = Code::new("E0742").card(
        "`void` and `never` are only allowed as a return type. A parameter or a variable cannot \
         have either type, and a variable cannot store the result of a `void` call.",
    );

    /// `Core\Program::implementing<T>()`'s type argument is neither an
    /// interface nor a class.
    ///
    /// `rule:programs/implementing`: the type argument is what gives
    /// `$module->register($this)` a static type, where
    /// [`E_METHOD_ON_ERASED_RECEIVER`]'s `object` is opaque and a shape
    /// describes data rather than methods. An interface and a class both give
    /// one. An enum, a shape or a scalar written here would enumerate
    /// something nothing could then be *called* on, so the answer would be an
    /// array nobody can use.
    ///
    /// Reported where the type argument is written, in the same pass that
    /// expands the call, because the expansion needs the answer anyway.
    pub const E_PROGRAM_TYPE_ARG_NOT_A_CLASS_OR_INTERFACE: Code = Code::new("E0743").card(
        "The type argument of `Core\\Program::implementing` must be an interface or a class. \
         Write the interface the classes implement or the class they extend, for example \
         `implementing<Module>()`.",
    );

    /// A class the enumeration would instantiate declares a constructor that
    /// takes arguments.
    ///
    /// `rule:programs/implementing`: "Each such class needs a no-argument constructor; a
    /// diagnostic names any that does not, and dependencies arrive through the
    /// type argument's own methods instead." The call expands to one `new`
    /// expression per listed class and there is no call site to write arguments
    /// at, so this is refused rather than defaulted — a constructor parameter
    /// with a default would otherwise make the enumeration silently pick it.
    ///
    /// The primary span is the *call*, not the offending declaration: the
    /// program being compiled is the one that asked for the enumeration, and
    /// the implementing class may be in a file this program only reached
    /// through `rule:programs/implementing`'s scan. The class is named in the message and in the help.
    pub const E_PROGRAM_IMPLEMENTOR_NEEDS_NO_ARGUMENT_CONSTRUCTOR: Code = Code::new("E0744").card(
        "`Core\\Program::implementing` creates each class it finds with no arguments, and this \
         class has a constructor that takes arguments. Remove the constructor's parameters, and \
         pass those values through a method of the interface or base class instead.",
    );

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
    pub const E_OPTION_SPELLING_TAKEN: Code = Code::new("E0745").card(
        "Two `#[Option]` parameters of this command use the same option name. A parameter's name \
         is its long option name unless `long:` gives another. Change the `short:` or `long:` of \
         one of them.",
    );

    /// A `#[Command]` method's parameter whose declared type has no conversion
    /// from `string`: an `#[Option]`, or a positional argument. A positional
    /// `array<string>` is admitted only as the last positional argument.
    ///
    /// `rule:tooling/commands-are-compiled`'s third compile error. A matched value's type comes from
    /// the parameter and the argument arrives as text, so a parameter no text
    /// can be converted into is an option that could never be given — and that rule
    /// takes `rule:security/route-capture-is-laundered-by-its-type`'s conversion roster unchanged, which is why an
    /// `array<int>`, a shape or a class other than `Core\Uuid` is refused here
    /// while an enum, a set of allowed values and `bool` are not.
    ///
    /// The primary span is the parameter rather than the attribute: the
    /// attribute is written correctly and it is the declared type that cannot
    /// answer it.
    pub const E_OPTION_TYPE_HAS_NO_CONVERSION: Code = Code::new("E0746").card(
        "A command line argument arrives as text, so each parameter of a `#[Command]` method needs \
         a type that text can be converted to. Use `string`, `int`, `uint`, `decimal`, `bool`, an \
         enum, a set of allowed values like `\"asc\"|\"desc\"`, or a class that implements \
         `Parses`, like `Core\\Uuid`. The last parameter without `#[Option]` can also be \
         `array<string>`. It gets all the remaining words.",
    );

    /// A `#[Route]` gives no `path`, no `method`, or neither.
    ///
    /// `rule:routing/route-attribute` marks only `name` optional, and a row of the route table is a
    /// path and a verb together: an attribute naming neither declares nothing
    /// and would be a route the author believes exists. Reported by the pass
    /// that builds the table rather than by the payload's roster check, for
    /// the reason `nvs_types::routes` states — a roster says what a field may
    /// hold, and *required* is a fact about the row.
    ///
    /// Both missing fields are named in one diagnostic: an author who wrote
    /// neither wrote one empty attribute, not two mistakes.
    pub const E_ROUTE_INCOMPLETE: Code = Code::new("E0747").card(
        "A `#[Route]` needs both a `path` and a `method`. Write for example \
         `#[Core\\Route(path: \"/users\", method: Core\\Http\\Method::Get)]`.",
    );

    /// Two `#[Route]`s declare the same verb and the same path shape.
    ///
    /// `rule:routing/routes-are-compiled-not-registered`'s duplicate-route error. It matches by *shape*, so
    /// `/users/{id}` and `/users/{userId}` are one route however they are
    /// spelled, and the same path under a different verb is not a duplicate at
    /// all. A question about the whole enumeration rather than about one
    /// declaration, so it is reported once every file has been walked, at the
    /// declaration that arrives second in load order.
    pub const E_DUPLICATE_ROUTE: Code = Code::new("E0748").card(
        "Two routes have the same HTTP method and the same path. The names of captures do not \
         count, so `/users/{id}` and `/users/{userId}` are the same path. Change or remove one of \
         the routes.",
    );

    // `E0749` is retired and is never reused: it refused two routes claiming
    // one `name`, and a route has no name now. It is named by its method, and
    // one method's routes sharing a path is `E_ROUTE_PATHS_ON_ONE_METHOD`.

    /// A `#[Route]`'s `path` is not one `rule:routing/path-grammar`'s grammar admits.
    ///
    /// One code for the whole grammar, because every way a path fails it is
    /// the same fact — the router cannot build a node out of this — and the
    /// message names which way it was. The four: a path that does not begin
    /// at the root, a segment that is neither a fixed segment nor a whole capture
    /// (`/u{id}`, `/{id}.json`), a capture naming nothing a parameter could
    /// be called, and a `{name?}` or `{name...}` written anywhere but last.
    ///
    /// The grammar's "at most once" and "never in one path together" need no rule of
    /// their own: one segment is last, so a second trailing form is already
    /// in a position it is refused at.
    pub const E_ROUTE_PATH_GRAMMAR: Code = Code::new("E0750").card(
        "This route path has invalid syntax. A path starts with `/`, and each segment is plain \
         text or one whole capture like `{id}`. A `{name?}` or `{name...}` capture is only allowed \
         as the last segment.",
    );

    /// A `{name}` capture names no parameter of the method it is attached to.
    ///
    /// `rule:security/route-capture-is-laundered-by-its-type`'s first compile error. A capture's value arrives *as* the
    /// parameter it is named after, so a capture with no parameter is a value
    /// with nowhere to go; the comparison is exact, per
    /// `rule:core-api/identifier-casing`, so
    /// `{userId}` and `$userid` are two names. The reverse is not an error: a
    /// parameter the path does not name is simply not the router's.
    pub const E_ROUTE_CAPTURE_UNBOUND: Code = Code::new("E0751").card(
        "A capture like `{id}` in a route path needs a method parameter with the same name, such \
         as `$id`. Names must match exactly, including upper and lower case.",
    );

    /// A capture's parameter is declared at a type no path segment converts
    /// to.
    ///
    /// `rule:security/route-capture-is-laundered-by-its-type`'s second compile error, over the roster
    /// `nvs_types::commands::converts_from_string` holds for both this and
    /// `rule:tooling/commands-are-compiled`'s `#[Option]` — one list, read twice. A `{name...}` is
    /// the one capture that roster does not answer for: `rule:routing/path-grammar` hands a catch-all
    /// over as the single `tainted string` it was read as, nothing about it
    /// having been checked, so it binds a `string` and no other type.
    ///
    /// The primary span is the parameter, as with
    /// [`E_OPTION_TYPE_HAS_NO_CONVERSION`]: the path is written correctly and
    /// it is the declared type that cannot answer it.
    pub const E_ROUTE_CAPTURE_TYPE_HAS_NO_CONVERSION: Code = Code::new("E0752").card(
        "A route capture or a `#[Query]` value arrives as text, so its parameter needs a type \
         that text can be converted to. Use a type like `string`, `int`, an enum or `Core\\Uuid`. \
         A `{name...}` capture must be a `string`.",
    );

    /// A `{name?}` capture is bound to a parameter with no default.
    ///
    /// `rule:routing/path-grammar`: an optional capture matches one whole segment or none,
    /// and the default is what makes the absent case *well-typed* rather than
    /// nullable by accident — the router never invents a `null` for a
    /// parameter whose type does not admit one. Both sites are named, the
    /// parameter first, because the fix is a default rather than a rewritten
    /// path.
    pub const E_OPTIONAL_CAPTURE_NEEDS_DEFAULT: Code = Code::new("E0753").card(
        "An optional capture like `{page?}` can be missing from the URL, so its parameter needs a \
         default value. Write for example `int $page = 1`.",
    );

    /// `Core\Router::url`, `urlAbsolute` or `urlSigned` links to a method that
    /// has no `#[Route]`.
    ///
    /// `rule:routing/link-target-and-params-are-checked`: a method reference
    /// written in the call is resolved to its route while compiling, so this
    /// is reported at the argument rather than at the call, which is where the
    /// reference is. The table it is checked against is the whole program's,
    /// which is why the check runs after every file has been walked and not
    /// where the call is typed: the route may be declared in a file
    /// `rule:programs/implementing`'s scan reaches later.
    pub const E_LINK_METHOD_HAS_NO_ROUTE: Code = Code::new("E0754").card(
        "The method given to `Core\\Router::url` has no `#[Route]`. Pass the method that has \
         the route, or add a `#[Route]` to this method.",
    );

    /// A `Core\Router::url` `$params` array literal covers none of some capture the
    /// named route's path declares.
    ///
    /// `rule:routing/link-target-and-params-are-checked`: a `$params` array that does not cover the route's
    /// captures is a compile error. Only a `{name?}` may be absent — its
    /// segment simply is not emitted — so every `{name}` and `{name...}` the
    /// path writes needs a key of that name. A `$params` that is not an array
    /// literal is not checked at all: there are no keys to read, and the rule asks
    /// nothing of a computed one.
    pub const E_ROUTE_LINK_MISSING_PARAM: Code = Code::new("E0755").card(
        "The `$params` of `Core\\Router::url` must have a key for each capture in the route's \
         path. Add the missing key. Only an optional capture like `{page?}` may be left out.",
    );

    /// A derived field's declared type is not in its format's type map.
    ///
    /// `rule:core-classes/derive-property-list`'s codec-reachable set for `#[Json\Derive]` and `rule:core-classes/db-column-types`'s type map for `#[Db\Derive]`, refused at the *declaration* that
    /// wrote it rather than at the `decodeAs<T>` or `queryAs<T>` that later
    /// runs. The two maps disagree — `bytes` is a `BLOB` column and has no
    /// JSON spelling, a nested class is a JSON object and no column at all —
    /// so the message names which one answered. It is the genuinely
    /// unmapped types this names and not the reachable ones `nvs_stdlib::json`
    /// still owes a decoder; the two are told apart in `nvs_types::derive`,
    /// which owns that split.
    pub const E_DERIVE_FIELD_NOT_CODEC_REACHABLE: Code = Code::new("E0756").card(
        "The derive attribute cannot convert a property of this type. For example, `bytes` can be \
         a database column but has no JSON form. Change the property's type, or write the \
         conversion methods yourself.",
    );

    /// A class carrying a derive attribute hand-writes every codec half that
    /// attribute would generate.
    ///
    /// `rule:core-classes/derive-generates-what-is-missing`: the derive generates only what the class does not
    /// declare itself, so a class writing both `toJson` and `fromJson` gets
    /// nothing from `#[Json\Derive]` — and an attribute with no effect is a
    /// mistake rather than a no-op. `Core\Db\Codec` declares `fromRow` alone,
    /// so for `#[Db\Derive]` one written member reaches the same conclusion.
    /// Reported at the attribute, which is the thing to delete.
    pub const E_DERIVE_BOTH_HALVES: Code = Code::new("E0757").card(
        "This class already writes every method the derive attribute would generate, so the \
         attribute does nothing. Remove the attribute.",
    );

    /// A class carrying a derive attribute contributes no field to the codec.
    ///
    /// `rule:core-classes/derive-property-list`'s field list is the declared property list, so a class
    /// that declares no instance property — or skips every one it declares —
    /// derives an empty wire contract.
    /// `rule:core-classes/derive-generates-what-is-missing`'s rule about an attribute with no
    /// effect applies unchanged, and the fix is either a property or no
    /// attribute.
    pub const E_DERIVE_NO_FIELDS: Code = Code::new("E0758").card(
        "This class has no instance property for the derive attribute to convert. Add a property, \
         or remove the attribute.",
    );

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
    pub const E_ROUTE_LINK_UNKNOWN_PARAM: Code = Code::new("E0759").card(
        "A key in the `$params` of `Core\\Router::url` is not a capture of the route and not one \
         of its `#[Query]` parameters. The key is probably misspelled. Fix it, or remove it.",
    );

    /// An `#[Access]` gives no `allow`.
    ///
    /// `rule:attributes/access-payload` marks only `csrf` optional: the attribute exists to carry
    /// a decision, so one carrying none is exactly the omission
    /// `rule:attributes/access-is-a-required-sibling` refuses, written out
    /// instead of left out. Reported by the payload walk rather
    /// than by the roster check, for [`E_ROUTE_INCOMPLETE`]'s reason — a
    /// roster says what a field may hold, and *required* is not a fact a
    /// roster can state.
    pub const E_ACCESS_INCOMPLETE: Code = Code::new("E0760").card(
        "An `#[Access]` needs an `allow` value that says who may call the route. Write for \
         example `#[Core\\Access(allow: Role::Admin)]`.",
    );

    /// An `#[Access]`'s `allow` value is not an enum case or a class constant.
    ///
    /// `rule:attributes/access-payload`: the field's declared type is `mixed` because
    /// `rule:security/access-is-checked-for-presence-not-meaning` is a
    /// promise *not* to know what the decision means, so a narrower type would
    /// be a claim the compiler does not make. What stands in place of the type
    /// is that the value **names** something — a bare literal names nothing,
    /// so `allow: "admin"` is the magic string the attribute exists to
    /// replace, and that rule's whole guarantee is that the name resolves.
    pub const E_ACCESS_ALLOW_NOT_A_NAME: Code = Code::new("E0761").card(
        "The `allow` value of `#[Access]` must be an enum case or a class constant. Write \
         `allow: Role::Admin`, not a string like `allow: \"admin\"`.",
    );

    /// A `#[Route]` method carries no `#[Access]`.
    ///
    /// `rule:attributes/access-is-a-required-sibling`, and the whole of why
    /// that rule exists: there is no
    /// implicit `Public` and no configuration that supplies one, so a route
    /// that is open because its author decided so and one that is open because
    /// its author forgot are not the same text. Reported at the `#[Route]`
    /// rather than at the method, because the attribute is what makes the
    /// declaration owe a decision — a method with neither attribute owes
    /// nothing.
    ///
    /// The mirror of [`E_ACCESS_INCOMPLETE`]: that one is a decision that
    /// declares nothing, this one a route that declares no decision.
    pub const E_ROUTE_WITHOUT_ACCESS: Code = Code::new("E0762").card(
        "Every `#[Route]` method needs an `#[Access]` that says who may call it. For a route \
         anybody may call, write `#[Core\\Access(allow: Core\\Audience::Public)]`.",
    );

    /// One method carries two `#[Access]` attributes.
    ///
    /// `rule:attributes/access-payload`'s last rule. `rule:attributes/repeatable` makes every attribute repeatable and leaves the ambiguity to
    /// retrieval, which is exactly what cannot happen here: two decisions are
    /// two readings — every one of them, or any one of them — and choosing
    /// between them silently is the failure `rule:security/access-is-checked-for-presence-not-meaning` exists to prevent.
    /// Reported at the second, naming the first, because the first is the one
    /// an author reading the error is deciding whether to keep.
    pub const E_ACCESS_REPEATED: Code = Code::new("E0763").card(
        "A method can have only one `#[Access]` attribute. Delete one of them, and keep the one \
         that says who may call the method.",
    );

    /// A `csrf: false` beside a method whose every `#[Route]` names a verb with
    /// no check.
    ///
    /// `rule:security/csrf-is-on-by-default`'s opt-out, held to the thing it opts out of: the write
    /// check covers `Post`, `Put`, `Patch` and `Delete`, and the upgrade's origin
    /// check covers `Get`, so beside a method declaring none of those the field
    /// turns nothing off.
    /// Refused rather than ignored because a field that reads as a security
    /// decision and has no effect is exactly how one ends up pasted onto routes
    /// that never needed it — and a reader auditing those routes then has to
    /// re-derive which of them the runtime was ever going to check.
    pub const E_CSRF_WITHOUT_UNSAFE_VERB: Code = Code::new("E0764").card(
        "`csrf: false` has no effect here, because no `#[Route]` on this method uses `Get`, \
         `Post`, `Put`, `Patch` or `Delete`. Only those HTTP methods have a CSRF check. Delete \
         `csrf: false`.",
    );

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
    pub const E_QUERY_WITHOUT_ROUTE: Code = Code::new("E0765").card(
        "`#[Query]` works only on a parameter of a method that has a `#[Route]`. Add a \
         `#[Route]` to the method, or delete `#[Query]`.",
    );

    /// An `#[Option]` on a parameter of a method carrying no `#[Command]`.
    ///
    /// `rule:tooling/commands-are-compiled`'s marker, held to the declaration that reads it, exactly as
    /// [`E_QUERY_WITHOUT_ROUTE`] holds `rule:routing/a-query-parameter-is-declared-like-a-capture`'s. Reported from the walk
    /// over every method rather than from the command pass, which by
    /// construction sees only the methods a `#[Command]` selects.
    pub const E_OPTION_WITHOUT_COMMAND: Code = Code::new("E0766").card(
        "`#[Option]` works only on a parameter of a method that has a `#[Command]`. Add a \
         `#[Command]` to the method, or delete `#[Option]`.",
    );

    /// A `#[Command]` that names no command.
    ///
    /// `rule:tooling/commands-are-compiled` selects a command by the `name` a command line writes, so
    /// a row without one is reachable by nothing and the table pass is what
    /// discovers it — the same reading [`E_ROUTE_INCOMPLETE`] gives a
    /// `#[Route]` that gave no path, and the answer to that module's own
    /// question of whether `name` is required.
    pub const E_COMMAND_WITHOUT_NAME: Code = Code::new("E0767").card(
        "A `#[Command]` needs a `name`, which is the word you type to run the command. Write \
         `#[Core\\Command(name: \"deploy\", about: \"Push the current build\")]`.",
    );

    /// Two `#[Command]`s claiming one name — the first of `rule:tooling/commands-are-compiled`'s
    /// compile errors, and the one only the whole program's enumeration can
    /// answer.
    ///
    /// [`E_DUPLICATE_ROUTE`]'s sibling, reported over the collected table
    /// for its reason: a command line naming a word two methods answer to has
    /// no answer, and which of them ran would depend on the order the files
    /// were walked in.
    pub const E_DUPLICATE_COMMAND: Code = Code::new("E0768").card(
        "Two `#[Command]` methods have the same `name`. Each command needs its own name, so \
         rename one of them.",
    );

    /// A literal argument to an `rule:expressions/intrinsic-list-is-closed` intrinsic that its own grammar
    /// refuses.
    ///
    /// The whole point of that rule, as one code: the compiler read the
    /// constant with the parser the runtime would have used, so this is the
    /// throw the first request to reach the call would have taken, moved to
    /// `nvs check`. One code across the grammars rather than one per member —
    /// the message carries the parser's own words, and a reader searching for
    /// "malformed pattern" should not have to know which member made the
    /// refusal.
    pub const E_INTRINSIC_LITERAL_MALFORMED: Code = Code::new("E0769").card(
        "This text argument is not valid for the function it is passed to, for example a \
         regular expression with a syntax error. The compiler checks it with the same parser \
         the program uses when it runs. The message says what is wrong, so fix the text there.",
    );

    /// A `Core\Str::format` template or a `Core\Db` query, given as a string
    /// literal, that does not fit the arguments written beside it.
    ///
    /// Separate from [`E_INTRINSIC_LITERAL_MALFORMED`] because the string is
    /// *fine*: the mistake is in the pairing, which is `rule:expressions/intrinsic-list-is-closed`'s own
    /// reason for naming this member's placeholder check separately — it turns
    /// PHP's `printf` argument-mismatch bug family into a compile error.
    /// `rule:core-classes/db-compile-time-query-checking`'s placeholder count and its positional-vs-named consistency
    /// are the same question of a second grammar, so they are the same code:
    /// the message carries the reader's own words, per
    /// [`E_INTRINSIC_LITERAL_MALFORMED`]'s.
    pub const E_FORMAT_TEMPLATE_MISMATCH: Code = Code::new("E0770").card(
        "The placeholders in this `Core\\Str::format` template or `Core\\Db` query do not match \
         the arguments next to it. Every argument must be used by a placeholder, and every \
         placeholder needs an argument. The message says which one is wrong.",
    );

    /// An `#[Api]` that contradicts the code it annotates — `rule:attributes/api-adds-and-cannot-contradict`'s
    /// contradictions, under one code.
    ///
    /// One code rather than several because that rule states them as one sentence: the
    /// annotation *may add, and may not contradict*. Each of them is a
    /// different way for the same sentence to be false, so what distinguishes
    /// them is the message and the two spans it names, not a number an author
    /// would ever look up separately. The fix is the same in every case —
    /// correct the attribute, or correct the code it disagrees with.
    pub const E_API_CONTRADICTS_THE_CODE: Code = Code::new("E0771").card(
        "This `#[Api]` attribute does not agree with the code it is on, or its method has no \
         `#[Route]`. `#[Api]` may add information, but it may not contradict the code. Change \
         the attribute or the code so that they agree.",
    );

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
    /// Only a *literal* value is checked, exactly as
    /// `rule:routing/a-leftover-link-key-is-a-query-string` checks only a literal key — a computed value has nothing to read.
    ///
    /// The same code covers a literal the parameter's conversion refuses, such
    /// as `"x"` or `-3` for a `uint`
    /// (`rule:routing/link-target-and-params-are-checked`): that link is a
    /// `404` for the same reason, and the card already says so.
    pub const E_ROUTE_LINK_VALUE_NOT_IN_SET: Code = Code::new("E0772").card(
        "A value in the `$params` of `Core\\Router::url` is not one of the values its route \
         parameter allows. A link with this value leads to a `404` page. Use an allowed value, or \
         widen the type of the route parameter.",
    );

    /// A `secret`-qualified value reaches
    /// `rule:classes/graph-copy`'s graph copy —
    /// `rule:security/secret-crosses-no-boundary`'s `serialize()`-and-`spawn` sink, which that rule deliberately
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
    /// isolate rather than a script one — and `rule:core-classes/topic`'s
    /// `Core\Topic::publish`,
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
    pub const E_SECRET_CROSSES_A_BOUNDARY: Code = Code::new("E0775").card(
        "A `secret` value cannot be copied to another isolate or turned into bytes, for example \
         with `Core\\Serialize::encode`, the `args:` of `spawn script` or \
         `Core\\Topic::publish`. Pass a value that is not `secret`, or get the plain text with \
         `Core\\Secret::reveal` first.",
    );
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
    pub const E_INSTANCE_METHOD_CALLED_STATICALLY: Code = Code::new("E0778").card(
        "This method is not `static`, so it needs an object to run on. Call it as \
         `$object->method()`, or declare the method `static`.",
    );

    /// `$this` written where no receiver is in scope — a `static` method's
    /// body, a plain function's, or the file scope.
    ///
    /// Separate from [`E_UNDEFINED_VARIABLE`] because `$this` is never
    /// *declared* by anything a program writes: the fix is not an assignment
    /// but a different method, so a message about a missing declaration would
    /// send the reader looking for one to add.
    pub const E_THIS_WITHOUT_A_RECEIVER: Code = Code::new("E0779").card(
        "`$this` exists only inside a method that is not `static`. Here there is no object, for \
         example in a `static` method or a function. Move the code into a method that is not \
         `static`, or pass the object as a parameter.",
    );

    /// `throw` of a value that is not a `Throwable` — a scalar, an `array<T>`,
    /// an enum, or a class outside the `Core\Error` exception tree.
    ///
    /// The tree's root is what `catch` matches against and what
    /// `nvs_ir::lower::exception` builds a landing pad for, so an operand
    /// outside it has nothing to be caught by; the lowerer refuses to lower
    /// one at all.
    pub const E_THROW_OPERAND_NOT_THROWABLE: Code = Code::new("E0780").card(
        "`throw` works only with an object whose class implements `Throwable`, and this value \
         is not one. Throw a `RuntimeError`, a `LogicError` or a class of your own that extends \
         one of them.",
    );

    /// `clone` of a value that can hold no object — `rule:classes/clone-is-shallow` makes it "a
    /// new instance of `$x`'s class", and a scalar, an `array<T>` or an enum
    /// names no class to instantiate.
    ///
    /// An `array<T>` is the operand worth naming in the help rather than
    /// merely refusing: it is already copied by assignment (`rule:programs/memory-priority`'s
    /// copy-on-write), so the `clone` a reader reaches for is not missing but
    /// unnecessary.
    pub const E_CLONE_OPERAND_NOT_AN_OBJECT: Code = Code::new("E0781").card(
        "`clone` works only with an object, and this value is a number, a string, an array or \
         an enum. An array is copied when you assign it, so `$copy = $list;` is enough.",
    );

    /// A write to a `readonly` property from anywhere but the declaring
    /// class's own `constructor`, through `$this` — `rule:classes/readonly-is-written-once`'s
    /// contract for the modifier, "assigned exactly once, and that assignment
    /// happens during construction". A write through another instance is one
    /// even inside that constructor, because that instance is already built.
    ///
    /// Refused where the write is written rather than left to a run-time
    /// check, which is what makes the modifier mean anything at all: PHP
    /// admits an initializing write from any method of the declaring class and
    /// throws only on the second one, so the property is write-once by
    /// bookkeeping there and by the type system here.
    pub const E_READONLY_WRITE_AFTER_CONSTRUCTION: Code = Code::new("E0782").card(
        "A `readonly` property can only be set in the `constructor` of its own class, as \
         `$this->name = ...`. Set it there, or remove `readonly` if the value has to change later.",
    );

    /// A class names a `final` class as its superclass. PHP refuses the same
    /// declaration, and for the same reason: `final` is the author's statement
    /// that the class's behaviour is not extended, so an `extends` naming one
    /// is a contradiction rather than a widening.
    pub const E_FINAL_CLASS_EXTENDED: Code = Code::new("E0783").card(
        "This class extends a `final` class, and a `final` class cannot have subclasses. Remove \
         `final` from the parent class, or do not extend it.",
    );

    /// A class redeclares a method an ancestor declared `final`.
    ///
    /// Separate from [`E_FINAL_CLASS_EXTENDED`] because the two are separate
    /// promises — a class that may be extended can still hold a member that
    /// may not be replaced — and a reader fixing one is not fixing the other.
    pub const E_FINAL_METHOD_OVERRIDDEN: Code = Code::new("E0784").card(
        "This method replaces a method that a parent class declares `final`, and a `final` \
         method cannot be replaced. Give this method another name, or remove `final` from the \
         parent's method.",
    );

    /// `new` names a declaration that has no instances — an `abstract` class
    /// or an interface. Both leave members without a body, so the object it
    /// would allocate could not answer every call its own type admits.
    pub const E_ABSTRACT_INSTANTIATED: Code = Code::new("E0785").card(
        "`new` cannot create an object of an `abstract` class or of an interface. Create an \
         object of a class that extends or implements it.",
    );

    /// A class that is not `abstract` declares a method with no body.
    ///
    /// Separate from [`E_ABSTRACT_INSTANTIATED`] because it is the other
    /// direction of the same rule — that one refuses the instance, this one
    /// refuses the hole — and a class whose declaration is fixed here has no
    /// `new` to fix.
    pub const E_ABSTRACT_METHOD_IN_CONCRETE_CLASS: Code = Code::new("E0786").card(
        "This method has no body, but its class is not `abstract`. Give the method a body, or \
         declare the class `abstract`.",
    );

    /// A write to a property that declares a `get` hook and no `set` hook.
    /// The accessor pair is the whole of what such a property answers with,
    /// so a write has nothing to commit through.
    pub const E_GET_ONLY_HOOK_WRITE: Code = Code::new("E0787").card(
        "This property has a `get` hook and no `set` hook, so you cannot assign a value to it. \
         Add a `set` hook, or assign to the value the `get` hook reads.",
    );

    /// An `#[Access]` on a method carrying no `#[Route]`.
    ///
    /// `rule:attributes/access-is-a-required-sibling` makes the attribute a *sibling* of `#[Route]`, and
    /// [`E_ROUTE_WITHOUT_ACCESS`] is that sentence read in the other
    /// direction: a route must declare a decision. This one refuses the
    /// decision that guards no route — `rule:attributes/access-payload` promises the
    /// compiler will not interpret the `allow` value, so away from the route table there is
    /// nothing that ever reads it. The sibling of [`E_QUERY_WITHOUT_ROUTE`]
    /// and [`E_API_CONTRADICTS_THE_CODE`], which are the same mistake made
    /// with the other markers a `#[Route]` gives meaning to.
    pub const E_ACCESS_WITHOUT_ROUTE: Code = Code::new("E0788").card(
        "`#[Access]` works only on a method that has a `#[Route]`. Add a `#[Route]` to the \
         method, or delete `#[Access]`.",
    );

    /// A `#[Command]` method that is not `static`, or that returns something
    /// other than `void` or `uint`.
    ///
    /// `rule:tooling/commands-are-compiled`'s two facts about the declaration the attribute sits on,
    /// under one code for [`E_TEST_METHOD_SHAPE`]'s reason: they are one
    /// question — whether this declaration is a command handler — and an
    /// author fixing either is editing the same line.
    pub const E_COMMAND_METHOD_SHAPE: Code = Code::new("E0789").card(
        "A `#[Command]` method must be `static` and must return `void` or `uint`. A `uint` \
         return value is the exit code of the command.",
    );

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
    pub const E_SECRET_OUTPUT: Code = Code::new("E0790").card(
        "`echo`, `print` and `html` templates cannot write a `secret` value. This includes a string that contains \
         one, such as `\"Password: {$password}\"`. If the value really must be shown, get the \
         plain text with `Core\\Secret::reveal` first.",
    );

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
    pub const E_SECRET_ENCODED: Code = Code::new("E0791").card(
        "A `secret` value cannot be passed to `Core\\Json::encode` or `Core\\Queue::push`. If \
         one field really has to be in the document, call `Core\\Secret::reveal` on that field \
         only.",
    );

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
    /// Distinct from [`E_SINGLE_VALUE_TYPE_NOT_CONST`], which refuses the same
    /// declaration in *type* position under `rule:types/constant-in-type-position`'s narrower question
    /// ("is this a `string` or `int` single-value type"). An `array` constant is
    /// legal here and refused there, and after the array fold this code is
    /// down to the shapes no constant emitter exists for at all — another
    /// class's `const`, an enum case, and `Foo::class` nested inside a
    /// container.
    pub const E_CLASS_CONST_NO_CONSTANT_FORM: Code = Code::new("E0792").card(
        "The value of this class constant cannot be computed when the program compiles, \
         because it uses another class's constant, an enum case or `Foo::class`. Write the \
         value out directly, as a number, a string in quotes, or an array of them.",
    );

    /// `rule:types/callable-values`'s `(...)` naming a member whose parameter list a
    /// `callable` cannot carry — one declared `inout $x`, a variadic tail, or
    /// a `Core` shape parameter a call must write, such as `Core\Db::connect`'s
    /// settings. An options bag is not on the list: the reference stops before
    /// it, and the thunk passes its defaults.
    ///
    /// `rule:types/callable-is-the-only-function-type` gives `callable` no parameter list, so a call *through* one passes
    /// what it was written with and nothing else: there is no site that could
    /// know to stage a by-reference cell, and none that could know to collect
    /// a tail into the one array the callee reads that slot as. Both are
    /// therefore a type confusion in the callee rather than a wrong answer,
    /// which is why this is a refusal and not a run-time report.
    ///
    /// The `inout` half is [`E_ANON_FN_INOUT_PARAM`]'s rule reached by the
    /// other spelling — that code refuses the parameter where an anonymous
    /// function *declares* it, this one refuses naming a member that already has one.
    pub const E_METHOD_REF_UNFORWARDABLE: Code = Code::new("E0793").card(
        "`method(...)` cannot make a `callable` from a method with an `inout` parameter, a \
         variadic parameter (`...$rest`) or a required `{...}` settings object. Write an \
         anonymous function with `fn` that calls the method instead.",
    );

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
    pub const E_DYNAMIC_NEW_DIVERGENT_CONSTRUCTOR: Code = Code::new("E0794").card(
        "`new` through a `class<T>` value needs every class that extends `T` to accept the \
         arguments of `T`'s constructor. One of those classes declares a constructor that does \
         not. Change that constructor so it accepts the same arguments.",
    );

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
    pub const E_CLASS_REF_ARGUMENT_NOT_A_CLASS: Code = Code::new("E0795").card(
        "In `class<T>`, `T` must be a class or an interface. `class<int>`, an array type or an \
         enum is not allowed.",
    );

    /// `rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key`: a request member whose verb repeats an effect —
    /// `Core\Http\Client::post` — asking for retries without
    /// `retryIdempotencyKey`.
    ///
    /// Reportable while compiling because both halves are written: the verb is
    /// the member's own name, and `rule:core-api/shape-rules` R2 makes the options bag an
    /// anonymous object written at the call site. Distinct from [`E_UNKNOWN_OPTION`], which is the
    /// mistake of naming an option that does not exist; here every option
    /// named is real and it is the *absent* one that is the defect.
    pub const E_RETRY_WITHOUT_IDEMPOTENCY_KEY: Code = Code::new("E0796").card(
        "Retries for a request like `Core\\Http\\Client::post` need a `retryIdempotencyKey`, \
         because sending the request again can repeat what it does. Add \
         `retryIdempotencyKey: \"...\"` to the same options. The server receives it as the \
         `Idempotency-Key` header.",
    );

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
    pub const E_SECRET_LOGGED: Code = Code::new("E0797").card(
        "A `secret` value cannot be written to a log. Log something that identifies it \
         instead, such as its key name. If the value itself must be in the log, call \
         `Core\\Secret::reveal` on that field only.",
    );

    /// A `Core\Attributes` retrieval whose `$member`, given as a string literal, names no declared
    /// parameter or property of the target.
    ///
    /// `rule:attributes/structural-retrieval`'s last paragraph: a written `$member` is checked against
    /// the target's real declarations at the call site, the same
    /// inspection of a string literal that `rule:security/secret-sinks-refuse`'s sinks make. Only a
    /// *computed* `$member` falls back to the empty result that rule fixes,
    /// because there is no string left to check.
    ///
    /// Distinct from [`E_ATTRIBUTE_TARGET_NOT_A_DECLARATION`], which is the
    /// mistake of writing something that is not a declaration reference at
    /// all: here the target resolves and it is the member name inside it that
    /// reaches nothing. Reported instead of folding, since the fold a
    /// misspelling produces — `null`, or the empty array — is exactly what a
    /// correct retrieval of an absent attribute produces, and nothing later
    /// can tell the two apart.
    pub const E_ATTRIBUTE_MEMBER_NOT_DECLARED: Code = Code::new("E0798").card(
        "The member name passed to `Core\\Attributes` is not a parameter or property of the \
         target. Check the spelling of the name.",
    );

    /// `rule:types/property-key`: a `property<T>` whose argument is not a class —
    /// `property<int>`, an interface, an enum.
    ///
    /// A property key's values are the names of `T`'s public declared
    /// properties, so the argument has to be something that declares
    /// properties. An interface is refused with the rest and the rule says why
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
    /// filled twice — at `E0499` and here — and the types diagnostics continue
    /// in the `E08xx` band; this module's legend table and the paragraph
    /// under it say why.
    pub const E_PROPERTY_KEY_ARGUMENT_NOT_A_CLASS: Code = Code::new("E0799").card(
        "In `property<T>`, `T` must be a class with at least one public property. An interface, \
         an enum or a type like `int` is not allowed.",
    );
}
