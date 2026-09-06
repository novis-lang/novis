A `for` header's init clause holds **one typed local declaration, or a list of expressions, and never
a mix**:

```
for-init := local-decl | expr (',' expr)* | ε
```

`local-decl` is the ordinary typed local declaration in full — `int $i = 0`, `var $i = 0`, and any
other type the grammar spells — with exactly one binding and no comma tail. The condition and step
clauses are unchanged and stay expression lists: a step is a reassignment or an increment, both
expressions already, and a declaration there would re-declare on the second iteration.

The declaration form lowers to the same slot store the declare-above form lowers to, at the same
point in the same block, so a counter in the header costs nothing in time or space. Both spellings
work and neither is rewritten into the other; a counter read after its loop, or shared by two loops,
is the case that still wants the declaration above.
