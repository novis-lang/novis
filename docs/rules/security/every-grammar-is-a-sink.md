The library parses exactly four grammars — a regex pattern, a format template, a date pattern, and a
byte-packing format — and every one of them is an instruction under
`rule:security/sink-predicate`. So each of the four is a sink, and the count that fixes the library's
grammar surface doubles as the roster.

**No launderer is added for any of them, deliberately.** A grammar is written by the program, not
received by it, so the fix at a failing call site is to use a literal — which is folded at compile time
for all four (`rule:expressions/intrinsic-constant-arguments`) — or, for a genuinely dynamic template drawn from
a translation catalogue, `rule:security/assert-trusted` with its written reason.

A regex quoting member remains the one exception on the roster, because a regex is the one grammar
that routinely needs a runtime value *inside* it rather than *as* it
(`rule:security/regex-pattern-is-a-sink`).
