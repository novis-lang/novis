Novis has one type test, `$x is T`, and `instanceof` is refused where it is written: the diagnostic
names the rewrite — *write `$x is Request`; a class reference on the right is `$x is $cls`* — and the
converter performs it mechanically. PHP keeps both operators, and this is a deliberate divergence
taken for good in 0192: `is` asks every question the pair used to split, so a language with both would
be carrying a keyword for a special case of its own operator.

`rule:types/type-test` is the operator and owns its table. This rule owns everything about **either
keyword and PHP**, and it is the only place any of it is written.

## What a converted program meets

| PHP | Novis | why |
|---|---|---|
| `$x instanceof C` | `$x is C` | the two answer identically for a class name |
| `$x instanceof $cls` where `$cls` is a `string` | `E0496`, help *`as class<T>`* | a class reference is checked where it is made, not at the test (`rule:types/class-reference-sites`) |
| `$x instanceof $obj` | `E0496`, the same report | an object is not a class reference; `$obj::class as class<T>` is the spelling |
| `$x is C` used as an identifier | renamed | `rule:php-migration/let-and-is-are-reserved` |

The second and third rows are `E0496` on purpose: a string or an object on the right of `instanceof`
is a place where PHP resolves a name at run time, and the fix is a conversion the author picks the
type for.

## What `$x is $cls` means here, and why it is not PHP's

`$x is $cls` tests the class a value holds against the descriptor a `class<T>` carries, and narrows
its subject to `T` on the true edge (`rule:types/narrowing`). The value arm starts with `$`; every
other token after `is` starts a type. PHP's Pattern Matching RFC refuses a bare variable as a whole
pattern and so has no dynamic class test through `is` at all — the spelling is unclaimed there rather
than claimed differently, which is why taking it costs nothing a converted program can trip over.

## The RFC is not read again

PHP 8.6 deprecates `is` as an identifier to reserve it for pattern matching, and that RFC is in
discussion. **No session re-reads it, tracks its vote, or compares either keyword with PHP again.**
What was read once is frozen in 0192 § 1, and a session that finds itself weighing what PHP would do
with either word has left the rule.

The shapes a pattern grammar would need — object and array patterns, comparison patterns, pinning,
`match ($x) is {…}` — keep refusing as syntax Novis does not have, and **no diagnostic names an RFC**.
Any future pattern syntax is a Novis design question, opened by its own record and decided on Novis's
priorities.
