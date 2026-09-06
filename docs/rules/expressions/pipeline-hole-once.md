The hole is `$_`. It wears a `$` because it is a binding, and in this language a binding wears a `$`.
The parser turns `$_` into its own node before identifier casing sees a variable, so the rule that
rejects an all-underscore identifier does not fire on it.

**`$_` appears exactly once on a right side** — not at least once. Three refusals keep that
enforceable:

- a right side of `|>` containing no `$_`, whose diagnostic names the shape (`Str::trim($_)`) and,
  when the right side is first-class callable syntax or a closure value, says that this `|>`
  substitutes a hole rather than applying a callable;
- `$_` more than once on one right side, which names binding the value to a local instead;
- `$_` anywhere outside the right side of a `|>`, which has no meaning.

Exactly one hole is what makes substitution total: no temporary, no double evaluation, and an emitted
tree identical to the nested spelling's.
