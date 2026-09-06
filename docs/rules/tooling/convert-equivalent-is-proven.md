A branch may be **E** only when, for every input the converted program's type checker accepts, the
two programs agree on all of: values returned, bytes written to each sink, which `Throwable` escapes,
and the order of externally visible side effects — evaluated against the declared target dialect
(`rule:tooling/convert-php-front-end`), never against "PHP in general".

**Nothing is E because someone was confident.** An E branch names a differential case in
`tests/convert/` that runs the PHP fragment on the oracle build this repository already keeps and
the converted fragment under `nvs test`, and compares. CI refuses an E branch whose `proof` is
missing or whose case does not run. A branch claiming E across several dialects owes one case per
dialect; where no oracle exists for a dialect, the branch is D, not E.

A tier is usually a property of the **site**, not of the construct. `==` keeps its spelling
(`rule:expressions/one-equality-operator`) and is E when both operands are proven the same
non-`string` scalar or proven numeric, D otherwise with a `diverges` sentence per operand shape, and
has no E or D branch at all for a cross-type comparison, which is
`rule:expressions/disjoint-comparison-refused`'s compile error. `strlen($s)` is E when the argument
is proven `bytes` and D when it is `string`, because grapheme counting changes the number on any
non-ASCII input — a `TODO`, not a blocker.

Where inference cannot decide a type, **`mixed` is an E answer, not a divergence**: it is the one
unchecked position (`rule:types/unions-and-mixed`), which is exactly PHP's own discipline. The
accompanying `TODO` names the binding and says what it costs.
