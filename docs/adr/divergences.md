# Where Novis deliberately diverges from PHP

Novis uses PHP's syntax and does not promise PHP's semantics. This is the register of every place the
two are known to differ **on purpose** — one row per divergence, with the ADR that owns it.

It exists because the divergences were previously counted in prose, ADR by ADR: [0007](0007-explicit-type-system.md)
§ 7 listed nine, [0010](0010-enums-are-a-value-type.md) called itself "the tenth", `0011` "the eleventh",
`0012` "the twelfth", and every ADR after that added rows while saying only "joining the divergence list".
A running total kept in sixteen files is a total that is wrong in fifteen of them, and the count was.

**The linked ADR is the rule; this page is only the index.** A row here never states a mechanism, a
diagnostic or a migration path — those live in the ADR, which is also where the reasoning is.

Three things read this register and each is a reason to keep it complete:

- **[0080](0080-the-audience-nvs-is-built-for.md) § 3** forbids any document from implying that existing
  PHP runs. This is the page that makes that claim checkable instead of a matter of tone.
- **[0089](0089-convert-is-one-rule-table-with-two-modes.md)** tiers every `nvs convert` rule by whether
  the converted construct behaves identically. A row here is a rule that cannot be tier **E**.
- **M11's `.phpt` pass rate** must distinguish a failure from an intentional divergence, or it reads as
  regression. This is the list that distinguishes them.

A divergence that is a *removed function name* is not here — [docs/spec/02-php-migration.md](../spec/02-php-migration.md)
carries one row per PHP built-in and is the home for those.

## The type system

| PHP | Novis | Owner |
|---|---|---|
| a variable holds anything, and its type changes silently | every binding declares a type, fixed for its lifetime; `settype()` is rejected | [0007](0007-explicit-type-system.md) § 7 |
| a function, method or closure may omit its return type | mandatory on every one, `void`/`never` written out | [0007](0007-explicit-type-system.md) § 7 |
| reading an undefined variable warns and yields `null` | a definite-assignment error at check time | [0007](0007-explicit-type-system.md) § 7 |
| array keys are `int` or `string` | always `string`; `Arr::keys()` returns `array<string>` | [0007](0007-explicit-type-system.md) § 5 |
| one integer type | `int` and `uint`, reported distinctly, and `int ⊕ uint` does not compile | [0007](0007-explicit-type-system.md) § 4 |
| `PHP_INT_MAX + 1` becomes a `float` | throws `ArithmeticError` | [0007](0007-explicit-type-system.md) § 4 |
| a leading zero is octal, so `017` is fifteen | decimal; `017` is seventeen and `0o17` is the octal | [0007](0007-explicit-type-system.md) § 4 |
| `(int)9.9` is `9`; `$a[1.7]` is `$a[1]` | throws; rounding is `Core\Math::floor`/`round`, said out loud | [0007](0007-explicit-type-system.md) § 2 |
| `int` → `float` rounds silently above 2^53 | throws | [0007](0007-explicit-type-system.md) § 2 |
| `(int)"abc"` is `0` | the cast syntax does not parse; `"abc" as int` throws, `as ?int` is `null` | [0034](0034-legacy-cast-syntax-rejected.md), [0066](0066-nullable-conversion-operator.md) |
| `strlen()` counts bytes; character-aware length needs `mb_strlen()` | `string` is guaranteed UTF-8 and counts grapheme clusters; binary data is `bytes` | [0009](0009-string-and-bytes.md) |
| an enum case is a singleton object with methods, interfaces and `::from()`/`::cases()` | a closed, named integer type; a case is a compile-time constant | [0010](0010-enums-are-a-value-type.md) § 7 |
| money is `float`, `bcmath` strings or `gmp` | `decimal` is a scalar; `bcmath` and `gmp` are retired | [0054](0054-decimal-scalar-type.md) |
| `stdClass` and dynamic properties | `{a: 1}` builds a methodless instance with fixed fields; there are no dynamic properties | [0036](0036-anonymous-object-shapes.md) |

## Operators and expressions

| PHP | Novis | Owner |
|---|---|---|
| `==` juggles and `===` escapes it | one operator, `==`; `===`/`!==` do not parse, and two disjoint types do not compile | [0090](0090-one-equality-operator-and-disjoint-types-do-not-compile.md) |
| `<>` is a second spelling of `!=` | does not parse; `!=` is the only spelling | [0090](0090-one-equality-operator-and-disjoint-types-do-not-compile.md) § 1 |
| `"1" == "01"` is true; `==` on arrays ignores key order; `==` on objects walks properties | strings compare as text, arrays element-by-element in order, objects by identity | [0090](0090-one-equality-operator-and-disjoint-types-do-not-compile.md) § 3 |
| `<`/`>` on two objects walks declared properties | requires `Comparable`; otherwise a compile error | [0013](0013-comparable-interface.md) |
| `and`, `or`, `xor` are lower-precedence connectives | rejected; `&&`/`\|\|` are the only ones, and `xor` has no replacement | [0045](0045-and-or-xor-keyword-operators-rejected.md) |
| `$a + $b` unions two arrays | a compile error naming `Arr::underlay`; `array_merge`'s key-type rule has no equivalent | [0069](0069-array-combination-is-key-type-independent.md) |
| `(int)$x` casts | does not parse; `as` is the only conversion spelling | [0034](0034-legacy-cast-syntax-rejected.md) |
| no pipeline operator; PHP 8.5's `\|>` applies a callable | `\|>` substitutes the hole `$_` at parse time — the same token, a different operator | [0098](0098-pipeline-operator-is-a-hole-substituted-at-parse-time.md) |

## Declarations and the object model

| PHP | Novis | Owner |
|---|---|---|
| a member with no visibility keyword is `public` | every member declaration writes one; an omission is a compile error | [0094](0094-visibility-is-written-at-every-member-declaration.md) |
| identifiers may be cased any way | casing is checked and a mismatch is a hard error, with no suppression | [0029](0029-identifier-casing-is-checked.md) |
| `_name` is a convention; the constructor is `__construct` | no identifier may begin with `_`; the constructor is `constructor` | [0030](0030-no-leading-underscores-constructor-spelling.md) |
| class and function names resolve case-insensitively; `IF`/`TRUE` parse | every name resolves case-sensitively, and a reserved spelling is lower case only | [0062](0062-case-sensitivity-is-a-compiler-property.md) |
| `trait`, `use Trait;`, `insteadof` | no trait exists; interface default/private methods, plus `implements I by $field;` | [0043](0043-interface-default-methods-and-delegation-replace-traits.md) § 6 |
| `class_alias()`, `use X as Y;` | rejected; a declaration is reachable under exactly its own name | [0015](0015-no-name-aliasing.md) |
| a typed property may be declared-but-unset and throw on read | every constructor must definitely assign every property, checked at compile time | [0022](0022-definite-property-initialization.md) |
| `__get`/`__set`/`__call`/`__callStatic` intercept undefined access | no fallback exists; an undeclared member is an error, and `PropertyObserver` observes declared ones | [0014](0014-property-observer.md) |
| `__toString`, `__destruct`, `__isset`/`__unset`, `__debugInfo`, `__set_state` | `Stringable` replaces the first; the rest are removed, and `unset()` on a declared property is refused | [0028](0028-closing-the-remaining-magic-methods.md) |
| `__clone`, `__serialize`/`__unserialize`, `__sleep`/`__wakeup` | no copy hook of any kind, and `unserialize` accepts only Novis's own closed format | [0023](0023-clone-serialize-and-cross-boundary-copy.md) |
| `ArrayAccess`, `Countable`, and ~20 SPL iterator classes | `Iterable<T>`/`Iterator<T>` only; `$obj[$k]` on a non-array does not compile | [0053](0053-iteration-and-generators.md) § 3 |
| `yield from`, `send()`, `throw()` into a generator | a generator is a one-way lazy sequence, lowered to a state machine | [0053](0053-iteration-and-generators.md) §§ 4–5 |
| a closure inside a method always binds `$this`; `static fn` opts out | capture is implicit, by value, and `$this` is bound only when the body uses it | [0008](0008-static-and-global.md) § 4 |
| `function () use (&$y) {}` and two closure literals | one literal, `fn`, and no `use` clause at all | [0031](0031-callable-is-the-only-closure-type.md) |
| `callable` accepts strings, arrays and `__invoke` objects | only a closure; `$obj(...)` never resolves to a method | [0027](0027-callable-is-closures-only.md) |
| `list($a, $b) = $p;` | does not parse; `[...]` is the only destructuring spelling | [0050](0050-list-destructuring-spelling-rejected.md) |
| `new class { … }` declares an anonymous class | refused; a named class in the same file, or a closure | [README](README.md) § *Decisions taken at project start* |
| `catch (A \| B $e)` handles two classes in one clause | refused; two clauses, or one on the common ancestor | [README](README.md) § *Decisions taken at project start* |
| `$e` outlives its `catch` clause and is readable after the `try` | the binding ends with its clause; only the thrown value ever assigns it | [0007](0007-explicit-type-system.md) § 1's `catch` row |
| `public const X = 1;` takes the type of its value | a constant writes its type, as every other binding does | [README](README.md) § *Decisions taken at project start* |
| `namespace X { … }` and several namespaces per file | refused; `namespace X;` once, before any declaration | [README](README.md) § *Decisions taken at project start* |
| an `inout` (`&`) argument may be an element or a property | a local only; read it into one, pass it, store it back | [0107](0107-by-reference-parameters-are-spelled-inout-at-both-ends.md) § 5 |
| `<?php` opens code; `die` terminates | `<?nvs` and `exit` are the only spellings — and a `#!` first line opens code with no tag | [0049](0049-single-open-tag-and-single-exit-keyword.md), [0100](0100-against-python-nvs-claims-the-tool-that-gets-handed-over.md) § 3 |

## Scope, names and the standard library

| PHP | Novis | Owner |
|---|---|---|
| `global`, function-scope `static` | neither exists; state lives in the four places [0008](0008-static-and-global.md) § 2 lists | [0008](0008-static-and-global.md) |
| a function or constant may be declared at file scope | every callable is a method and every constant a class constant, built-ins included | [0011](0011-functions-and-constants-are-class-members.md) § 5 |
| `$_GET`, `$_POST`, `$_SERVER`, `$_SESSION`, `$_ENV`, `$GLOBALS`, `$argv` | no variable is host-populated; each is a `Core` accessor, and `$GLOBALS`/`$_REQUEST` have no replacement | [0012](0012-no-superglobals.md) § 9 |
| `include`, `include_once`, `require_once` | one construct, `require`, which throws and runs every time | [0021](0021-single-file-inclusion-construct.md) |
| `spl_autoload_register` resolves a name at run time | `autoload` maps a prefix to a root at compile time; there is no runtime loader | [0061](0061-compile-time-autoload-and-program-discovery.md) |
| ~1,900 global built-ins, with argument order and failure signalling varying per function | ~450 members across ~30 `Core` classes, all obeying one shape | [0063](0063-core-api-conventions.md) |
| `preg_*` on PCRE, where a pattern may backtrack exponentially | a linear-time engine by default; backtracking is opt-in by pattern and budgeted, and there is no `u` modifier | [0056](0056-regex-engine-policy.md) |
| `var_dump`, `print_r`, `var_export`, `json_encode`-as-a-debug-tool | one `Core\Debug::dump`, rendered by the sink in force, and it reaches a response body only in development mode | [0092](0092-one-diagnostic-record-three-renderings.md) |
| `exec`/`system`/`shell_exec`/backticks/`proc_open` | `Core\Process`, argv-only; there is no shell-string form and no flag that adds one | [0044](0044-core-process-argv-only-no-shell.md) |
| APCu, `shmop`, `sysv*` share memory across requests | no cross-request state except an explicit, capability-gated store | [0052](0052-closed-doors.md) § 3, [0059](0059-cross-request-state-is-explicit.md) |
| `eval`, `FFI`, `dl()`, stream wrappers and `phar://` | all four closed, with no ini flag and no trusted mode | [0052](0052-closed-doors.md) |
| `putenv`, `setlocale`, `bcscale`, `mb_internal_encoding`, `date_default_timezone_set` | no ambient process-global state; locale, scale and timezone are always explicit arguments | [0052](0052-closed-doors.md) § 3, [0063](0063-core-api-conventions.md) § 4 |
| a qualified name is relative to the current namespace, and a leading `\` forces the root | every name with a `\` is absolute; a leading `\` does not parse, and there is no fallback to the root for a short name | [0113](0113-a-qualified-name-is-absolute.md) |
| `use Foo\Bar;` also imports `Bar` as a prefix, so `Bar\Baz` reaches `Foo\Bar\Baz` | an import binds one whole short name and is never a prefix; the name itself is imported, or written in full | [0113](0113-a-qualified-name-is-absolute.md) § 1 |
| `continue` inside a `switch` behaves as `break` | `switch` owns `break` and nothing else; `continue` always means the innermost loop | [README.md](README.md) § *Decisions taken at project start* |

## Security defaults that change observable behaviour

These are divergences because a ported program behaves differently, not because a construct was removed.

| PHP | Novis | Owner |
|---|---|---|
| echoing a variable into HTML emits it raw | the HTML sink auto-escapes any non-`Markup` value; only a source literal converted `as Markup` writes raw | [0024](0024-taint-tracking-for-injection-sinks.md) § 5 |
| output to a terminal passes control bytes through | `ESC` and every other control byte is substituted with a visible glyph, uniformly | [0086](0086-core-cli-terminal-is-a-sink.md) § 1 |
| a cookie name is mangled (`.` and space become `_`) | matched byte for byte, with `__Host-`/`__Secure-` enforced by the runtime | [0095](0095-ambiguous-input-is-refused-never-repaired.md) § 3 |
| a malformed or ambiguous HTTP message is normalised and served | refused whole, in both directions, with no per-call opt-out | [0095](0095-ambiguous-input-is-refused-never-repaired.md) § 2 |
| an unterminated bidirectional control is passed through | a compile error in source, and substituted at every output sink | [0087](0087-unbalanced-bidi-is-rejected-at-every-boundary.md) |
| a response carries whatever headers the program wrote | secure headers, closed CORS and `Secure; HttpOnly; SameSite=Lax` cookies with nothing configured | [0074](0074-http-defaults-safe-and-finite.md) |
| an outbound call may wait forever | there is no spelling for an unbounded wait | [0074](0074-http-defaults-safe-and-finite.md) § 5 |
| a fatal error is catchable through `set_error_handler`-adjacent paths | a resource-limit report is not a `Throwable` at the type level | [0020](0020-error-escalation-ladder.md) § 0 |
