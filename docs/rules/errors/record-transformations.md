Four transformations are properties of the **record**, applied when it is built, before any
rendering sees it. Every rendering therefore inherits identical answers and none may weaken one.

| Transformation | Rule |
|---|---|
| Redaction | a property whose *declared* type carries `secret` becomes a Redacted node |
| Control bytes | C0 except `LF`/`TAB` becomes its U+2400 Control Picture, `DEL` becomes `␡`, a C1 code point becomes `�` |
| Bidi | an unterminated directional control becomes `�`; a balanced one passes through |
| Elision | depth and per-node length caps, replacing what is cut with an Elided node naming how much |

Substitution applies to the JSON rendering too, where framing already makes it unnecessary for
safety. Uniformity is the point: a value must not read differently depending on which rendering
someone is looking at, or the renderings stop being views of one record.

**This is what closes log forging for the plaintext rendering.** A human-readable line is not
`"$k=$v"` concatenation — it renders nodes whose control bytes are already substituted, so a
newline inside a tainted value cannot forge an entry. That property is the condition on a
human-readable target existing at all.

Elision belongs to the model precisely so the renderings agree on what was cut, and a cycle is an
identity rather than a `*RECURSION*` marker, so the HTML rendering can link the repeat and the JSON
rendering can emit a reference.

Tainted data flows into a record freely, because the record frames it and **the rendering is what
makes it safe**. There is no raw escape hatch, and no rendering may be selected by an argument.
