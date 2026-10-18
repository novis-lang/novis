Every parameter of every `Core` member may be written by name at the call site — the `$name` the member's
signature writes in `nvs meta --json`, and the trailing bag under the one name `options` — under exactly the
rules a user-declared method's parameters follow (`rule:types/arrays`): written order is evaluation order, a
name fills its own slot, a defaulted parameter may be skipped, a positional after a name is refused, and a
name never reaches a variadic tail.

A parameter's **name is compatibility surface**, versioned where its type is, so renaming one is a
breaking change. That is the price of the feature and it is paid deliberately: every published signature
shows its parameter names, so the surface is public already, and a surface a user's own method has that a
`Core` member lacks is one more rule to learn. The name lives once, on the registry row beside the type,
and the reference card looks it up rather than repeating it (`rule:core-api/reference-card`). Nothing on
the request path changes — a `name:` resolves while checking and the helper ABI is untouched. No member's
shape, order, parameter list or bag moved for this: a name labels a slot that already exists, never adds
one.

- **Memory:** per process, one static string slice per registry row, in read-only data. Nothing per
  request or per task.

**Declined:**
- **Positional-only `Core` members, with the options bag as their only by-name surface.** It kept
  parameter names out of the compatibility surface, but every signature publishes its names anyway, and
  the grammar and checker rules for named arguments already exist for user-declared methods
  (`rule:types/arrays`), so all it bought was an asymmetry between the two.
