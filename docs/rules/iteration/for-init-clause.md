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

Every other binding position has a type slot — a local at its declaration, a `foreach` key and value,
a destructuring pattern — and the init clause is where a counter is declared in the shape a reader
writes first. Without the slot, `for (int $i = 0; …)` does not parse, and the reader gets a run of
errors about a `;` and a counter typed `mixed` that name nothing about `for`. A reader who knows
`foreach ($rows as int $row)` can guess this syntax and be right. The cost is one more production in
the statement grammar, spent on simplicity of the language surface.

**Declined:**
- **Keeping the expression-only clause and improving the diagnostic.** It makes the gap legible
  without making it defensible, since `foreach` has the slot. The diagnostic half is kept as
  `rule:iteration/for-init-refusals`, for the shapes that still do not parse.
- **A comma-separated list of declarations**, `for (int $i = 0, int $j = 0; …)`. One declaration
  declares one binding everywhere else, so this would be the one statement that declares two
  (`E0125`, `rule:iteration/for-init-refusals`).
