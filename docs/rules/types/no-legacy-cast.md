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
(`rule:types/conversion`), and a PHP file carrying a legacy cast needs that one mechanical rewrite
before it parses.
