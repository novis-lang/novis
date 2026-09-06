A `switch` label and a `match` arm are compared against the subject by
`rule:expressions/equality-semantics`, and by nothing else. There is one comparison in the language,
so there is one here.

A label or arm whose static type is disjoint from the subject's is
`rule:expressions/disjoint-comparison-refused`'s compile error, at the label rather than at the
`switch`.

`match (true) { … }` is unaffected: each arm is a `bool`, tested against `true`.

Truthiness has no part in this. A `switch` subject and a `match` subject are compared, never tested
for truth, and `rule:expressions/truthy-positions`'s six positions do not include either.
