PHP-compatible observable behaviour is the second priority, so a departure from it is never discovered
later: every one is listed, PHP's behaviour beside Novis's. Almost every one is reachable in PHP only
*because* a binding somewhere is untyped, and is what a declared type answers instead.

The list, by where each is stated: array keys are always `string` (`rule:types/arrays`); `int` and
`uint` are distinct types, reported distinctly (`rule:types/uint`); every binding is declared and its
type fixed, `settype()` rejected (`rule:types/declaration`); `(int)"abc"` is refused as syntax, `"abc"
as int` throws and `"abc" as ?int` is `null` (`rule:types/no-legacy-cast`, `rule:types/conversion`);
integer overflow, a fractional value where an integer is wanted, and an `int` too wide for a `float`
all throw (`rule:types/arithmetic`); a return type is mandatory on every function, method and block-bodied anonymous function,
`void` or `never` stated when there is no value (`rule:types/declaration`); absent storage never reads
as a zero value (`rule:php-migration/absent-storage-is-never-a-zero-value`); a declared type answers
`->` before the program runs
(`rule:php-migration/a-declared-type-answers-before-the-program-runs`); an element write needs storage
to write back into (`rule:php-migration/an-element-write-needs-storage-to-write-back-into`); a body
never falls off its end (`rule:php-migration/a-body-never-falls-off-its-end`); and a spread carrying a
string key after an integer-looking one is accepted where PHP fatals, the variadic tail being the
array itself (`rule:types/arrays`).

**One divergence is not of that kind**, and it is listed here so the exception is not mistaken for an
oversight: PHP's own class-test operator is refused where it is written and `$x is T` is the one type
test (`rule:php-migration/one-type-test`). No untyped binding makes it reachable — it is a choice of
spelling, taken because one operator answers what PHP splits across two. It costs a converted program
a mechanical rewrite and never a silent change of meaning, since the refused word does not compile.

The consequence to plan around: the imported `.phpt` corpus passes at a **structurally lower** rate
than a compatibility-first design would, and a failure in one of these classes is intentional
divergence, not a bug. The tracked number distinguishes the two, or it reads as regression.
