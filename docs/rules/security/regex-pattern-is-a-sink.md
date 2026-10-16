A regex **pattern** requires the plain, unqualified `string`. A user-supplied pattern is both a
denial-of-service vector and a logic-injection vector: a pattern an attacker controls can be made to
match anything, which turns a validation check into an approval.

**There is no laundering member for it**, because there is no meaningful way to make an arbitrary
attacker-authored pattern safe. A program that genuinely needs one uses
`rule:security/assert-trusted` and says why. That is the shape every grammar on the roster takes
(`rule:security/every-grammar-is-a-sink`); the regex quoting member is not a counter-example, because
it escapes a value to sit *inside* a pattern rather than to *be* one.

The **subject** may be tainted, and contagion applies unchanged: a substring matched out of a tainted
subject is tainted (`rule:security/taint-propagation`). The denial-of-service half is answered
separately, by tiering the pattern onto a linear-time engine and budgeting the backtracking tier.
