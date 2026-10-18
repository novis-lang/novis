Four tests narrow a variable:

- `$x == null` and `$x != null` on a `?T`. Each narrows on the branch where the value is not `null`.
  `!` swaps the two branches, so `if (!($x == null))` narrows its block.
- `$x is C` on a `?C`, a union, `object` or `mixed`. Only the branch where the test is true narrows.
  After `if ($pet is Cat) { return …; }`, `$pet` is still `Dog|Cat`, and `$pet->speak()` does not
  compile (`E0477`).
- `$x == "read"` or `$x == Mode::Read` on a set of allowed values or a union of enum cases. The right
  side must be a value written in the code or an enum case. A comparison with a variable narrows
  nothing. The branch where the test is false keeps the full type. It is not narrowed to the
  values that are left.
- Each arm of `match (true)` and each `case` of `switch (true)` narrows its own code when its label
  is one of the tests above. `default` narrows nothing.

A ternary's two arms and the right side of `&&` and `||` are narrowed like the branches of an `if`.
What a test proves ends with the expression. After `$d != null ? $d->speak() : "none"`, `$d` is a
`?Dog` again.

The block of `if ($a != null && $b != null)` sees both variables narrowed. After
`if ($a == null || $b == null) { return …; }`, the code below also sees both narrowed. The `else`
of an `&&` and the block of an `||` narrow nothing, because only one of the two tests is known
there.

A write to the variable inside the branch ends the narrowing. A property is narrowed when it is
`readonly`, and so is every property before it in the path, such as `$app->config->parent`. Any
other property, such as `$d->friend`, is never narrowed, because a call can change it between the
test and the read. Copy it to a local variable first. A `->` on a `?C` that is not narrowed does not
compile (`E0459`). `$d?->speak()` needs no test: when `$d` is `null`, the result is `null`.
