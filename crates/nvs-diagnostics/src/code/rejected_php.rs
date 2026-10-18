//! The codes for a PHP construct that Novis does not have, `E02xx`.
//!
//! A new code here takes the highest number in this file plus one, never a
//! hole: a hole is a retired number. A code outside `E02xx` fails the
//! crate's tests.

codes! {
    range = "E02";

    // Novis accepts PHP 8.5 syntax as a *pragmatic* superset. These constructs
    // are recognised — so the diagnostic can be precise and suggest a
    // replacement — and are then rejected. Most parse first; the ones a
    // *lexical* rule refuses (`E_RESERVED_SPELLING_CASE`,
    // `E_IDENTITY_OPERATOR_UNSUPPORTED`, `E_ANGLE_NOT_EQUAL_UNSUPPORTED`) are
    // named where they are recognised, which is the lexer. A few are shapes
    // PHP refuses as well and admits here by nothing but omission; they are in
    // this band because the answer is the band's own: recognise the shape and
    // name its rewrite. The rule each one cites places it here. See docs/spec.
    /// `eval()`: Novis compiles ahead of execution.
    pub const E_EVAL_UNSUPPORTED: Code = Code::new("E0201").card(
        "`eval()` is not available in Novis. Every program is compiled before it runs, so code \
         in a string cannot run. Put the code in a file and `require` it, or run it with \
         `spawn script`.",
    );
    /// `$$name` and `${$name}`: defeats name resolution and type inference.
    pub const E_VARIABLE_VARIABLE: Code = Code::new("E0202").card(
        "A variable variable such as `$$name` or `${$name}` is not allowed. The name of a \
         variable must be written in the code. Use an `array<T>` with one entry per name instead.",
    );
    /// `goto`: makes the control-flow graph unstructured.
    pub const E_GOTO_UNSUPPORTED: Code = Code::new("E0203").card(
        "`goto` is not available in Novis. Write the jump as a loop, an early `return`, or a \
         `bool` variable that the code checks.",
    );
    /// `global $x`: use a parameter or an explicit process-scoped binding.
    pub const E_GLOBAL_UNSUPPORTED: Code = Code::new("E0204").card(
        "`global` is not available in Novis. A function cannot read a variable from outside \
         itself this way. Pass the value as a parameter, or make it a `static` property or a \
         `const` on a class.",
    );
    /// `extract()`: introduces bindings whose names are not known statically.
    pub const E_EXTRACT_UNSUPPORTED: Code = Code::new("E0205").card(
        "`extract()` is not available in Novis. It creates variables whose names are only known \
         when the program runs. Destructure the array, or read each value by its key.",
    );
    // `E0206` is retired and is never reused: Novis has no free functions, so
    // a call into a PHP extension is `E0320` like any other.
    // `E0207` is retired and is never reused: `Core\Regex` runs lookbehind,
    // backreferences, recursion and `\K`, and no check names a construct it lacks.
    /// `settype($x, ...)`: no assignment, operator or call can change what a
    /// binding's type is.
    pub const E_SETTYPE_UNSUPPORTED: Code = Code::new("E0208").card(
        "`settype()` is not available in Novis. The type of a variable never changes after it \
         is declared. Convert the value with `as` and put the result in a new variable.",
    );
    /// `static $x = ...;` inside a function: there is no function-scope
    /// storage class — see `rule:statements/static-is-a-member-modifier`.
    pub const E_STATIC_LOCAL_UNSUPPORTED: Code = Code::new("E0209").card(
        "A `static` variable inside a function is not allowed. In Novis, `static` is only \
         written on a class member. Use a `private static` property on the class, or pass the \
         value as a parameter.",
    );
    /// `static function`/`static fn`: an anonymous function already captures
    /// `$this` only if it uses it, so `static` has nothing left to mean here.
    pub const E_STATIC_ANON_FN_UNSUPPORTED: Code = Code::new("E0210").card(
        "`static` cannot be written before an anonymous function. An anonymous function uses \
         `$this` only if its body uses it, so the word is not needed. Remove `static`.",
    );
    /// A PHP superglobal (`$_GET`, `$_SERVER`, `$GLOBALS`, `$argv`, …): no
    /// variable is ever populated by the host — see `rule:statements/no-host-populated-variables`.
    pub const E_SUPERGLOBAL_UNSUPPORTED: Code = Code::new("E0211").card(
        "PHP variables such as `$_GET`, `$_SERVER` and `$argv` do not exist in Novis. Nothing \
         fills a variable for you. Read the value from a `Core` class, and the message names \
         the one to use.",
    );
    /// `use Path\To\Name as Other;`: an import cannot be renamed — see
    /// `rule:statements/nothing-gets-a-second-name`. `use Shop\Order::Status;`
    /// is the same refusal: a member import would give an enum in a class body a
    /// second short name (`rule:enums/class-scoped-enum`).
    pub const E_IMPORT_ALIAS_UNSUPPORTED: Code = Code::new("E0212").card(
        "An import cannot be given a second name with `as`. Use the name the class was declared \
         with, or write its full path in the code. An import also cannot name an enum inside a \
         class, as `use Shop\\Order::Status;` does. Import the class and write `Order::Status`.",
    );
    /// `function foo() { ... }` outside any class: a function must be a
    /// method — see `rule:classes/no-free-functions-or-constants`.
    pub const E_TOPLEVEL_FUNCTION_UNSUPPORTED: Code = Code::new("E0215").card(
        "A function cannot be declared outside a class. Every function in Novis is a method. \
         Put it in a class as a `public static function`.",
    );
    /// `const FOO = 1;` outside any class: a constant must belong to a
    /// class — see `rule:classes/no-free-functions-or-constants`.
    pub const E_TOPLEVEL_CONST_UNSUPPORTED: Code = Code::new("E0216").card(
        "A constant cannot be declared outside a class. Every constant in Novis belongs to a \
         class. Declare it as a `public const` on the class it belongs to.",
    );
    /// `namespace Core;` (or anything nested under it) in user source:
    /// `Core` is reserved for built-ins — see `rule:core-api/reserved-namespace`.
    pub const E_RESERVED_CORE_NAMESPACE: Code = Code::new("E0217").card(
        "The namespace `Core` is reserved for the built-in library. Your code cannot declare it, \
         or any namespace inside it. Choose a different namespace name.",
    );
    /// `enum Name implements Iface { ... }`: an enum declares only cases and
    /// an optional backing type — see `rule:enums/no-class-machinery`.
    pub const E_ENUM_IMPLEMENTS_UNSUPPORTED: Code = Code::new("E0218").card(
        "An enum cannot implement an interface. An enum contains only its cases and an optional \
         backing type. Put the behavior in a `static` method on another class.",
    );
    /// `enum Name: string { ... }`: no `string` backing, only `int`/`uint` —
    /// see `rule:enums/no-class-machinery`.
    pub const E_ENUM_STRING_BACKING_UNSUPPORTED: Code = Code::new("E0219").card(
        "An enum cannot be backed by `string`. The backing type can be `int` or `uint`, or you \
         can leave it out.",
    );
    /// A method, property, class constant or trait use inside an `enum`
    /// body: an enum declares only cases and an optional backing type — see
    /// `rule:enums/no-class-machinery`.
    pub const E_ENUM_MEMBER_UNSUPPORTED: Code = Code::new("E0220").card(
        "An enum cannot contain a method, a property, a class constant or a trait. An enum \
         contains only its cases and an optional backing type. Move this code to a separate \
         class.",
    );
    /// `include`, `include_once`, or `require_once`: Novis keeps exactly one
    /// same-frame inclusion construct, `require` — see `rule:statements/require-is-the-only-inclusion-construct`.
    pub const E_INCLUDE_FAMILY_UNSUPPORTED: Code = Code::new("E0221").card(
        "`include`, `include_once` and `require_once` are not available in Novis. Use \
         `require`. It throws an error when the file is missing, and it runs the file every \
         time it is reached.",
    );
    /// An anonymous `function (...) { ... }` expression, with or without a
    /// `use` clause: `fn` is the only anonymous function syntax — see
    /// `rule:types/anonymous-function`.
    pub const E_FUNCTION_EXPRESSION_UNSUPPORTED: Code = Code::new("E0222").card(
        "An anonymous `function (...) { ... }` is not allowed. `fn` is the only way to write an \
         anonymous function. Write `fn(...) => expr` for one expression, or `fn(...) => { ... }` \
         for a block.",
    );
    /// `use ($y)` on an anonymous function: capture is always implicit and by
    /// value, so there is no clause to write — see `rule:types/implicit-capture`.
    pub const E_ANON_FN_USE_UNSUPPORTED: Code = Code::new("E0223").card(
        "An anonymous function has no `use` clause. It captures every outer variable its body \
         reads, by value, automatically. Remove the `use (...)` clause.",
    );
    /// `use (&$y)` on an anonymous function specifically: by-reference capture
    /// has no replacement syntax — see `rule:types/implicit-capture`.
    pub const E_ANON_FN_USE_BY_REF_UNSUPPORTED: Code = Code::new("E0224").card(
        "An anonymous function cannot capture a variable by reference, as `use (&$x)` does. It \
         captures by value only. Store the shared value in an object property, and use the \
         object inside the function.",
    );
    /// PHP's legacy `(T)expr` cast syntax — `as` is the only conversion
    /// spelling. See `rule:types/no-legacy-cast`, which amends `rule:types/conversion`.
    pub const E_LEGACY_CAST_UNSUPPORTED: Code = Code::new("E0225").card(
        "The cast syntax `(int)$x` is not allowed. Write `$x as int`. It throws an error if the \
         value cannot be converted.",
    );
    /// PHP's `and`/`or`/`xor` keyword operators — `&&`/`||` are the only
    /// logical connectives. See `rule:expressions/no-keyword-logical-operators`.
    pub const E_LOGICAL_KEYWORD_UNSUPPORTED: Code = Code::new("E0226").card(
        "The keyword operators `and`, `or` and `xor` are not available in Novis. Use `&&` for \
         `and` and `||` for `or`. For `xor` on two `bool` values, write `$a != $b`.",
    );
    /// `trait Name { … }`, `use TraitName, ...;` inside a class body, or
    /// `insteadof` anywhere: traits do not exist — an interface
    /// default/private method replaces shared behavior, and
    /// `implements Interface by $field;` replaces shared state. See
    /// `rule:classes/no-traits`. One code for the whole shape: there is no
    /// trait `use { ... }` adaptation grammar to diagnose more finely, since
    /// traits do not exist at all. The default-method/delegation conflict is
    /// [`E_INTERFACE_MEMBER_CONFLICT`].
    pub const E_TRAIT_NOT_SUPPORTED: Code = Code::new("E0227").card(
        "Traits do not exist in Novis. For shared methods, give an interface a default method. \
         For shared state, write `implements Interface by $field;` in the class.",
    );
    /// `die`, in any position `exit` is also accepted: Novis keeps exactly one
    /// process-termination keyword. See `rule:statements/exit-is-the-only-termination-keyword`.
    pub const E_DIE_UNSUPPORTED: Code = Code::new("E0228").card(
        "`die` is not available in Novis. Use `exit`. It takes the same optional status or \
         message.",
    );
    /// The `<?php` open tag: Novis keeps exactly one code-mode open tag,
    /// `<?nvs` (plus the short-echo `<?=`). See `rule:statements/nvs-is-the-only-open-tag`.
    pub const E_PHP_OPEN_TAG_UNSUPPORTED: Code = Code::new("E0229").card(
        "The `<?php` open tag is not available in Novis. Write `<?nvs` instead. The code after \
         it stays the same.",
    );
    /// `list(...)` as a destructuring target: Novis keeps exactly one
    /// destructuring spelling, `[...]`. See `rule:expressions/bracket-destructuring`.
    pub const E_LIST_DESTRUCTURING_UNSUPPORTED: Code = Code::new("E0230").card(
        "`list(...)` cannot be used to destructure a value. Write `[...]` instead. The elements \
         inside the brackets are written the same way.",
    );
    /// A reserved lexical spelling written in anything but lower case —
    /// A reserved spelling written in a case Novis does not have: `<?NVS`
    /// rather than `<?nvs`, and a duration's unit, `30S` rather than
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
    pub const E_RESERVED_SPELLING_CASE: Code = Code::new("E0231").card(
        "A reserved word must be written in lower case. Write `<?nvs`, not `<?NVS`. Write a \
         duration unit as `30s`, not `30S`.",
    );
    /// PHP's `===`/`!==` — Novis keeps exactly one equality operator, `==`, and
    /// its negation `!=`. See
    /// `rule:expressions/one-equality-operator`. This is the one construct in this band the *lexer* reports rather
    /// than the parser: there is nothing to parse precisely here, so the
    /// three characters are consumed, named, and lexed as the two-character
    /// operator so the rest of the file still reports its own problems.
    pub const E_IDENTITY_OPERATOR_UNSUPPORTED: Code = Code::new("E0232").card(
        "The operators `===` and `!==` are not available in Novis. Use `==` and `!=`. They \
         never convert either value before they compare.",
    );
    /// A `class`, `interface` or `enum` declaration written inside a function
    /// body, a property hook or a nested block. PHP declares such a type when
    /// the statement *runs*, so whether the name exists depends on control
    /// flow; Novis resolves every type name against a static table built before
    /// any code runs. See `rule:classes/a-type-is-declared-at-file-scope`,
    /// and `nvs_types::locals`' module
    /// doc for the walk that reports it.
    pub const E_NESTED_TYPE_DECLARATION_UNSUPPORTED: Code = Code::new("E0233").card(
        "A `class`, `interface` or `enum` cannot be declared inside a function or a block. \
         Every type is declared before any code runs. Move the declaration to the top level of \
         the file.",
    );
    /// An `unset()` operand that is not an array element of a named holder —
    /// `unset($x)` on a bare local, or a subscript of a temporary such as
    /// `unset(rows()["k"])`. `rule:classes/unset-is-refused-on-a-property` keeps `unset()` for exactly one job,
    /// removing an array entry: a binding is declared with a type and
    /// definitely assigned (`rule:types/declaration`), so there is no "undefined again"
    /// state for a local to return to, and a temporary has nothing for the
    /// separated array to be written back into
    /// (`rule:types/an-element-write-needs-storage-to-write-back-into`). An operand that is
    /// a *declared* property is [`E_UNSET_ON_PROPERTY`] instead, which owns
    /// that half of the same section.
    pub const E_UNSET_TARGET_NOT_AN_ELEMENT: Code = Code::new("E0234").card(
        "`unset()` only removes an entry from an array, as in `unset($rows[\"key\"])`. It \
         cannot remove a variable. The array must be stored in a variable or a property.",
    );
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
    /// behind the handle is unknown. Neither needs this spelling, and `rule:types/anonymous-object` already refuses its anonymous-object twin, the computed key
    /// `{[$expr]: 1}`.
    ///
    /// `rule:types/property-key-access` carves out the one exception and moves the report with it:
    /// `$obj->$key` is admitted where `$key`'s type is a `property<T>` the
    /// receiver satisfies — a set of names checked where the `as` was written,
    /// so nothing request-controlled picks a field — and every other operand is
    /// still this code, reported by `nvs_types` rather than by the parser,
    /// since the operand's type is what decides and a parser sees none.
    pub const E_DYNAMIC_MEMBER_NAME: Code = Code::new("E0235").card(
        "A member name cannot be computed, as in `$obj->$name` or `$obj->{$expr}`. Write the \
         name of the property or method in the code. `$obj->$key` is allowed only when `$key` \
         has the type `property<T>`.",
    );
    /// `@expr` — PHP's error-suppression prefix. There is nothing for it to
    /// suppress: `rule:errors/escalation-ladder`
    /// makes every runtime failure a `Throwable` propagated by checked return
    /// (`rule:errors/propagation`), not a
    /// diagnostic printed alongside a value, and `rule:core-api/removals` already lists
    /// `@` among the constructs that decision closes. `try`/`catch` is the
    /// replacement, and it is the only one.
    pub const E_SUPPRESSION_UNSUPPORTED: Code = Code::new("E0236").card(
        "The `@` operator does not exist in Novis. An error is always thrown as an exception. \
         Catch it with `try` and `catch`.",
    );
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
    /// [`E_ANON_FN_USE_BY_REF_UNSUPPORTED`] each name a rule rather than a
    /// spelling, and `inout` is not what replaces any of them.
    pub const E_BY_REFERENCE_MARKER_RETIRED: Code = Code::new("E0237").card(
        "`&` does not mark a parameter or a variable as a reference in Novis. Write `inout` \
         before the type, and again before the argument where you call the function. A \
         function cannot return a reference.",
    );
    /// `use App\Models\{User, Post};` — PHP's group-use form. One `use`
    /// statement imports exactly one name, so the short name a file introduces
    /// is always written on a line of its own
    /// (`rule:statements/nothing-gets-a-second-name`), and
    /// [`E_IMPORT_ALIAS_UNSUPPORTED`] is its sibling refusal on the other half
    /// of the same statement.
    pub const E_IMPORT_GROUP_UNSUPPORTED: Code = Code::new("E0238").card(
        "An import cannot name a group of classes, as `use App\\Models\\{User, Post};` does. \
         Write one `use` statement for each name.",
    );
    /// PHP's `case Name = 1;` enum-body spelling. Novis writes a case as a
    /// bare `Name = 1,` in a comma list, with no `case` keyword — see
    /// `rule:enums/declaration`. Raised on the `case` keyword itself, once,
    /// rather than as an [`E_ENUM_MEMBER_UNSUPPORTED`] cascade: that code is
    /// for a *member* in an enum body and its "move this to a separate class"
    /// help is the wrong answer here, since the case belongs in the enum and
    /// only its spelling is wrong.
    pub const E_PHP_ENUM_CASE_UNSUPPORTED: Code = Code::new("E0239").card(
        "An enum case is not written with the `case` keyword. Write each case as `Name = 1,` in \
         a list separated by commas, with no `;`.",
    );
    /// A leading `\` on a name — `\App\Models\User`, `use \App\Models\User;`,
    /// `namespace \App;`. A name containing a `\` is already read from the
    /// root, so the prefix has no work left to do, and accepting it would be
    /// the second spelling
    /// `rule:statements/a-leading-separator-does-not-parse`
    /// exists to remove. PHP rejects the `namespace` spelling too; the other
    /// two it accepts, which is what made the token mean three different
    /// things by position.
    pub const E_LEADING_BACKSLASH_UNSUPPORTED: Code = Code::new("E0240").card(
        "A name cannot start with `\\`. A name that contains a `\\` is already read from the \
         root namespace. Remove the first `\\`.",
    );
    /// `<>` for inequality, PHP's inherited second spelling of `!=`.
    /// `rule:expressions/one-equality-operator` makes
    /// `==` and `!=` the whole set, so this is the same decision
    /// [`E_IDENTITY_OPERATOR_UNSUPPORTED`] reports and not a lexical accident;
    /// it is separate from that code because the fix is a different edit and
    /// the reason is spelling rather than semantics — `<>` means exactly what
    /// `!=` means.
    pub const E_ANGLE_NOT_EQUAL_UNSUPPORTED: Code = Code::new("E0241").card(
        "The operator `<>` is not available in Novis. Use `!=`. It is the same comparison.",
    );
    /// A `try` block with neither a `catch` clause nor a `finally` — `try { …
    /// }` alone, which guards nothing and is the shape a deleted clause leaves
    /// behind. PHP refuses it too
    /// (`rule:statements/a-try-has-a-clause-and-a-catch-names-one-class`).
    pub const E_TRY_WITHOUT_CLAUSE: Code = Code::new("E0242").card(
        "A `try` block needs a `catch` or a `finally` clause. Add `catch (Throwable $e) { ... }` \
         to handle an error, or `finally { ... }` for code that always runs.",
    );
    /// The braced `namespace X { … }` form, a second `namespace X;` in one
    /// file, and a `namespace X;` after a declaration it would not cover.
    /// `rule:security/authority-is-the-enclosing-namespace`
    /// keys authority on the enclosing namespace, so a file that is two
    /// namespaces is a file whose authority is a function of the line number.
    /// The rewrite is one file per namespace, declared `namespace X;`.
    pub const E_BRACED_NAMESPACE_UNSUPPORTED: Code = Code::new("E0243").card(
        "A namespace cannot be written as a block with braces, and a file has only one \
         namespace. Write `namespace X;` once, before any declaration. Put a second namespace \
         in a second file.",
    );
    /// `new class { … }`: an anonymous class is a nested declaration in
    /// expression position, with no name for the static class table to hold —
    /// the same refusal a conditionally declared class gets, for the same
    /// reason. The rewrite is a named class in the same file, or an anonymous
    /// function (`rule:types/anonymous-function`).
    pub const E_ANONYMOUS_CLASS_UNSUPPORTED: Code = Code::new("E0244").card(
        "An anonymous class, `new class { ... }`, is not allowed. Declare a named class in the \
         same file and create it with `new`. If the class has only one method, use an anonymous \
         function written with `fn`.",
    );
    /// `catch (A | B $e)`, PHP's multi-class clause. The binding carries one
    /// static type (`rule:types/declaration`'s `catch` row), so a clause naming two classes has no type to give
    /// it; the rewrite is one clause per class, each with its own variable
    /// name, or one clause naming a class they all extend. The expression form
    /// refuses the same shape with the same words
    /// (`rule:expressions/catch-expression`).
    pub const E_CATCH_UNION_TYPE_UNSUPPORTED: Code = Code::new("E0245").card(
        "A `catch` clause names one class, so `catch (A | B $e)` is not allowed. Write one \
         `catch` clause for each class, or catch a class that they all extend.",
    );
    /// `public const LIMIT = 9;`, PHP's untyped class constant. Every other
    /// binding in the language writes its type
    /// (`rule:types/declaration`) and a
    /// constant is no different: the type read off the folded value is a
    /// guess the declaration never made, it is unavailable to an interface
    /// constant at all, and where the value has no constant form there is
    /// nothing to guess from.
    pub const E_CONSTANT_WITHOUT_TYPE: Code = Code::new("E0246").card(
        "A class constant must declare its type. Write the type before the name, as in \
         `public const int LIMIT = 9;`.",
    );
    /// `let` or `is` written as a name, which PHP 8.6 deprecates and Novis
    /// refuses outright (`rule:statements/let-and-is-are-reserved`). Both
    /// are reserved for a construct that does not exist — the family `eval`,
    /// `goto` and `list` are already in — so the spelling stays available and
    /// the help names the living one: `var` declares an inferred local, `is`
    /// tests and `as` converts. The rewrite is a rename.
    pub const E_RESERVED_FOR_FUTURE_USE: Code = Code::new("E0247").card(
        "`let` and `is` are reserved words and cannot be used as a name. Rename it. To declare \
         a variable without writing its type, use `var`.",
    );
    /// `$x instanceof Foo` — PHP's type test, written where a Novis operator
    /// may stand (`rule:types/one-type-test`).
    ///
    /// Novis has one type test and it is `is`, so the word is a keyword only
    /// to be refused here: read as an ordinary identifier it would arrive at
    /// name resolution as a constant that resolves to nothing, and the reader
    /// would be told about a name rather than about the operator they wrote.
    /// The parser consumes the right operand with it, so a site costs this one
    /// diagnostic.
    ///
    /// The rewrite is mechanical and the help carries both halves of it: a
    /// written class becomes `$x is Request`, and a class held in a binding
    /// becomes `$x is $cls`, which is the same site `rule:types/type-test`
    /// § *The value arm* gives (`rule:types/one-type-test`). A program never
    /// changes meaning silently over it, because the refused word does not
    /// compile.
    pub const E_INSTANCEOF_IS_NOT_AN_OPERATOR: Code = Code::new("E0253").card(
        "`instanceof` is not an operator in Novis. The type test is `is`. Write \
         `$x is Request`, or `$x is $cls` when the class is in a variable.",
    );
    /// A property declared inside an `interface` body. An interface declares
    /// behaviour and constants and no state
    /// (`rule:classes/interfaces-declare-no-state`): the body grammar is the
    /// class's, so the declaration parses, and it is refused where it is
    /// written rather than dropped by the layout pass — which is what left a
    /// clean `nvs check` followed by a codegen failure on the first read.
    pub const E_INTERFACE_PROPERTY_UNSUPPORTED: Code = Code::new("E0254").card(
        "An interface cannot declare a property. An interface has only methods and constants. \
         Require a method that each class writes, or declare a typed constant that the class \
         overrides.",
    );
    /// `return $value;` inside a `constructor`, which PHP 8.6 deprecates and
    /// this refuses (`rule:classes/a-constructor-return-carries-no-value`).
    /// The object under construction is the result and nothing else can be. A
    /// bare `return;` still leaves early and
    /// `rule:classes/definite-property-initialization` goes on checking that
    /// path, so only the value is named; the rewrite is dropping it.
    pub const E_CONSTRUCTOR_RETURN_CARRIES_A_VALUE: Code = Code::new("E0248").card(
        "A `return` in a constructor cannot return a value. The new object is always the \
         result. Write `return;` to leave early, or remove the `return`.",
    );
    /// `public readonly int $n = 1;`, which PHP 8.6 allows and this refuses
    /// (`rule:classes/a-readonly-property-declares-no-default`).
    /// `readonly` is one assignment during construction, so a property whose
    /// single assignment is its own declaration-site default is a per-instance
    /// constant — and `const` already spells one (`rule:types/class-constant`).
    /// The help names moving the value into the constructor and dropping
    /// `readonly` as the other fix.
    pub const E_READONLY_PROPERTY_WITH_DEFAULT: Code = Code::new("E0249").card(
        "A `readonly` property cannot have a default value. Assign the value in the \
         constructor. For a value known when you write the class, use a `const` or remove \
         `readonly`.",
    );
    /// A `return` written inside a `finally` block, which PHP 8.6 deprecates
    /// for removal and this refuses
    /// (`rule:statements/no-return-leaves-a-finally`). It replaces whatever
    /// the region was leaving with, including a throw in flight — the one
    /// construct where an unhandled exception vanishes with no handler
    /// anywhere. The help names the two rewrites the rule leaves: change the
    /// result in a `catch`, or write it after the region.
    pub const E_RETURN_LEAVES_A_FINALLY: Code = Code::new("E0250").card(
        "A `return` inside a `finally` block is not allowed, because it would hide an error \
         that is being thrown. Return from the `try` or a `catch` block, or after the whole \
         `try` statement.",
    );
    /// A `break` or `continue` inside a `finally` block whose level reaches
    /// past the loops and `switch`es the block itself opened
    /// (`rule:statements/no-return-leaves-a-finally`). Leaving the block
    /// discards what the region was leaving with, which is
    /// [`E_RETURN_LEAVES_A_FINALLY`]'s objection; a loop written wholly inside
    /// the block keeps both spellings, so only the reaching level is named.
    pub const E_BREAK_LEAVES_A_FINALLY: Code = Code::new("E0251").card(
        "A `break` or `continue` cannot jump out of a `finally` block, because it would hide an \
         error that is being thrown. Move the code it skips to after the whole `try` statement.",
    );
    /// PHP's `use function Foo\bar;` and `use const Foo\BAZ;`. There is no
    /// free function and no free constant to import
    /// (`rule:classes/no-free-functions-or-constants`), so the statement names
    /// a kind of thing the language does not have — which is what the help
    /// says, pointing at the class member the name is written as instead.
    /// Raised on the keyword, once, rather than left to the generic parse
    /// error that follows reading `function` as the imported short name.
    pub const E_IMPORT_OF_FUNCTION_OR_CONST_UNSUPPORTED: Code = Code::new("E0252").card(
        "`use function` and `use const` are not available in Novis. Every function is a method \
         and every constant belongs to a class. Import the class and write `Class::name`.",
    );
}
