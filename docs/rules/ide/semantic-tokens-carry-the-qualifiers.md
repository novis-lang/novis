The token types M4B emits are each chosen because the grammar structurally cannot answer them:
`namespace`, `class` (with `defaultLibrary` for a `Core` class, so the standard library is visibly not
user code), `interface`, `enum`, `enumMember`, `type` (an alias), `method`, `property`, `parameter`,
`variable`, `typeParameter` — and two modifiers of Novis's own, **`tainted` and `secret`**, so a qualified
value is visibly qualified at every use site rather than only where it was declared.

That pair is why this layer is built at M4B rather than M10: `rule:security/tainted-qualifier`'s and
`rule:security/secret-qualifier`'s whole model is that a value carries a qualifier through the program, and
an editor that shows it is the cheapest teaching surface the language has. "The qualifier is visible" is
verified as "the token carries the modifier", never as a colour.

The legend the client registers must equal the legend the server declares. A mismatch silently colours
everything one token type off, which no unit test on either side alone can see, so the extension-host run
proves it.
