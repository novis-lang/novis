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

That is why the exception costs no safety. What a checked conversion protects against is a declared
type changing silently, and a condition changes none. Without it, every "is this set" test — `if
($rows)`, `while ($line)`, `$user && $user->active`, `!$items` — would need `!= ''`, a count or an
`as bool` at every branch. The cost is one question to hold while reading a condition: whether this
position is one of the six. The list is short, fixed and syntactically obvious.

**Declined:**
- **No truthiness anywhere, so every condition must already be `bool`.** It puts the ceremony back at
  every branch, and safety does not need it.
- **Truthiness for `null` and `bool` only, with other scalars needing `as bool`.** It draws a line
  inside the table with no principled edge: `if ($rows)` is as common as `if ($maybeNull)`.
- **Truthiness in `if`, `while` and the ternary, but `bool` operands for `&&`, `||` and `!`.** The
  same value would be bare in one place and refused two tokens later, and `$user && $user->active`
  is as common as a bare `if`.
