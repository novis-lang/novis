`(int)expr`, `(uint)expr`, `(float)expr`, `(string)expr`, `(bool)expr`, `(array)expr` and
`(object)expr` are all rejected at parse time. The parser still recognises the shape — `(` a
scalar/`array`/`object` type keyword `)` — so that it can emit `E0225` naming the exact `as` spelling
to use, but it produces an error node; there is no cast node in the AST.

```nvs
(int)$x        // rejected — "use `$x as int` — it throws instead of silently truncating"
(string)$x     // rejected — "use `$x as string` — it throws instead of silently truncating"
```

The diagnostic fires only for that `(` *keyword* `)` shape in an operand position; the type keywords
keep their ordinary meaning everywhere else, and the rejected form still consumes its operand, so
`(int) !$x` leaves nothing dangling. `$x as int` is the only conversion spelling
(`rule:types/conversion`).

The cast would have been a second spelling of one operation, which
`rule:statements/nothing-gets-a-second-name` refuses everywhere: it adds a second thing to teach and a
second question for every review, and buys nothing once both spellings check and throw alike. `as`
is the one kept because every other conversion is written that way, `int → uint` included, which
has no cast. The cost is that a PHP file with a cast anywhere does not parse until each one is
rewritten to `as`, a mechanical change since the operand and the target are both known from the
syntax.

**Declined:**
- **`(int)$x` as a permanent alias for `$x as int`.** It is the second spelling above.
- **A warning first and the error later.** Every refused PHP construct is a hard error from the start,
  since a warning is only for code that runs as written (`rule:core-api/casing-has-no-suppression`), and
  a staged removal would be the one exception.
- **The cast kept for `array` and `object` only.** Every target has the same `as` spelling, and
  keeping two of seven has no principled line.
