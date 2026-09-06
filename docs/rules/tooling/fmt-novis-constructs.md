Each construct with no PER precedent has one layout, chosen once:

- A `tainted` or `secret` qualifier sits one space before the type it qualifies, and before a leading `?`:
  `tainted ?string $x`. The grammar already fixes that order; this is only the spacing.
- `lateinit` takes its place at the end of the modifier order (`rule:tooling/fmt-base-style-is-per`).
- An `fn` closure is an expression, so its body brace stays on the line of its parameter list and return
  type, as PER already lays out an anonymous function: `fn (int $x): int { return $x + 1; }`. With a
  multi-line body only the closing brace gets its own line.
- A `match` expression puts each arm on its own line unless the whole arm list was written on one, and a
  multi-line list ends in a trailing comma.
- An object literal or shape type on one line has one space inside each brace, `{a: 1, b: 2}`; across
  lines it is one field per line, indented one level, with a trailing comma.
- Enum cases are one per line when the author wrote them that way, with a trailing comma when multi-line.
- Attributes need no rule, since Novis has no annotation syntax.

Several of these have exactly one contributor and no convention to defer to. Changing one later is a
breaking rewrite of every formatted file, the same cost class casing already accepted.
