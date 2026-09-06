Exactly six syntax positions test a value's truthiness instead of requiring it to already be `bool`:
`if`/`elseif`'s condition, `while`/`do…while`'s condition, `for`'s middle clause, the ternary and
elvis condition, and the operands of `&&`, `||` and `!`. Each accepts a value of **any** type —
`mixed`, a union, a scalar, an array, an object, an enum case, `callable` — with no diagnostic for
not already being `bool`, and answers by `rule:expressions/truthy-table`.

The list is closed and nothing narrower gets the exception. Assigning into a `bool`-typed parameter,
property, return or local still needs an explicit `as bool` or a comparison; `==` and `match` still
compare rather than test (`rule:expressions/switch-match-equality`); `??` and `?->` test null against
not-null, which is a different axis and untouched.

A truthiness test converts nothing. It produces no value of a different type that can be read back,
assigned or passed on — it answers "branch or don't", freshly, every time — so a declared type never
changes here.
