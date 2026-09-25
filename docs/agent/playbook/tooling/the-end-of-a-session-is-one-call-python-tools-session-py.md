- **The end of a session is one call: `bun nv session --wrap <file>`.** Write one markdown
  file whose `## ` headings are instructions — `## plan-edit: <Field>`, `## playbook: <Heading>`,
  `## handoff`, `## commit: <paths>` once per slice, `## status` — and it applies all of them in a
  fixed order, or refuses one as broken and writes *nothing*. `--check` first says what the tree
  still owes, including any plan field whose prose names a conformance or differential count the
  tree contradicts. [until: gone tools/nv/cmd/session.ts:plan-edit]
