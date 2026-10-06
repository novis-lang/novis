Every member of every `Core` class obeys all twenty shape rules below. A proposed member that cannot is a
design bug rather than an exception, and the rules are decided before the library is written because a
surface of ~450 members is only learnable if the eleventh member is predictable from the first ten.

| # | The rule |
|---|---|
| R1 | The subject is parameter 1, always (`rule:core-api/subject-first`) |
| R2 | Then required arguments in dataflow order, then at most one trailing optional anonymous object (`rule:core-api/options-bag`), every parameter callable by name (`rule:core-api/parameters-are-callable-by-name`) |
| R3 | Nothing mutates and nothing takes a reference (`rule:core-api/nothing-mutates`) |
| R4 | Failure throws; absence is `?T` (`rule:core-api/failure-throws`) |
| R5 | A fixed verb lexicon, and a closed ban list (`rule:core-api/verb-lexicon`) |
| R6 | Symmetric operations get symmetric names (`rule:core-api/symmetric-names`) |
| R7 | Members are full words (`rule:core-api/members-are-full-words`) |
| R8 | One range convention, `(offset, ?length)` (`rule:core-api/one-range-convention`) |
| R9 | Callbacks receive `($value, $key)` (`rule:core-api/callback-receives-value-and-key`) |
| R10 | Haystack before needle, subject before pattern — R1's corollary |
| R11 | No mode strings; an enum instead (`rule:core-api/no-mode-strings`) |
| R12 | Units are types (`rule:core-api/units-are-types`) |
| R13 | A `string` member never takes an encoding argument (`rule:core-api/no-encoding-argument`) |
| R14 | Anything with a lifetime is an object (`rule:core-api/a-lifetime-is-an-object`) |
| R15 | One name, one signature (`rule:core-api/one-name-one-signature`) |
| R16 | A domain class is a singular noun (`rule:core-api/a-domain-class-is-a-singular-noun`) |
| R17 | One paradigm per operation; nothing is reachable two ways (`rule:core-api/one-paradigm-per-operation`) |
| R18 | A domain class's statics never mirror an object's own methods (`rule:core-api/one-paradigm-per-operation`) |
| R19 | Scalars and `array<T>` never gain methods (`rule:core-api/no-methods-on-scalars-or-arrays`) |
| R20 | No mutable/immutable twin types (`rule:core-api/no-mutable-immutable-twins`) |

Three properties of the language make the conventions these rules exclude unavailable rather than merely ugly:
arrays are copy-on-write values, so a by-reference mutator has no performance argument left; types are
declared and checked, so a `false` return and an `int` flag mask throw away what the compiler already
knows; and there is no ambient state for a member to read (`rule:core-api/no-ambient-state`).

What this costs is familiarity. A developer who knows `sort($a)`, `strtotime` and `ob_start` finds none of
them under that name, and ports each call by finding the `Core` member that does the job.
