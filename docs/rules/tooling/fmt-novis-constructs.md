Each construct with no PER precedent has one layout, chosen once:

- A `tainted` or `secret` qualifier sits one space before the type it qualifies, and a nullable type's
  `?` is written in front of the qualifier: `?tainted string $x`, `secret tainted bytes $y`. The grammar
  already fixes that order — a qualifier takes a scalar or a shape, and `?` wraps the qualified type
  rather than sitting inside it (`rule:security/tainted-qualifier`) — so this is only the spacing.
- `lateinit` takes its place at the end of the modifier order (`rule:tooling/fmt-base-style-is-per`).
- An `fn` closure is an expression, so its body brace stays on the line of its parameter list, return
  type and `=>`, as PER already lays out an anonymous function: `fn(int $x): int => { return $x + 1; }`.
  With a multi-line body only the closing brace gets its own line.
- A `match` expression puts each arm on its own line unless the whole arm list was written on one, and a
  multi-line list ends in a trailing comma.
- An object literal or shape type on one line has one space inside each brace, `{ a: 1, b: 2 }`; across
  lines it is one field per line, indented one level, with a trailing comma. An empty one has no inside
  to space and stays `{}`.
- Enum cases are one per line when the author wrote them that way, with a trailing comma when multi-line.
- A markup literal's body is never touched — not reflowed, not re-indented, not re-quoted — exactly as a
  heredoc body and an inline-HTML region are not (`rule:tooling/fmt-quotes`,
  `rule:core-classes/html-literal`). Only its surroundings are laid out, so the bytes between the
  backticks survive formatting unchanged and `nvs fmt` stays idempotent over a template.
- A `?>` that begins its line is indented to the depth of the block it sits in — the column a statement
  there would start at — so the markup after it can start from the code around it
  (`rule:ide/a-template-region-gets-the-editors-services-and-formatter`). Only its leading whitespace
  moves, and that whitespace is code, so nothing the program prints changes: a `?>` is never moved onto
  or off a line, and the markup after it is the editor's to lay out, never `nvs fmt`'s.
- Attributes need no rule, since Novis has no annotation syntax.

Several of these have exactly one contributor and no convention to defer to. Changing one later is a
breaking rewrite of every formatted file, the same cost class casing already accepted.
