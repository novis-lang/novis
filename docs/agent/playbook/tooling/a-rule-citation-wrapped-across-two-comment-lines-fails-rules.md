- **A `rule:` citation wrapped across two comment lines fails `bun nv rules --check` from a file you never
  touched.** `docs/agent/goals/` had split
  `rule:ide/the-extension-refuses-a-binary-it-does-not-understand` over two `#` lines, so the checker
  read the first half as a rule id that does not exist and `bun nv rules --render` has exited 1 ever since —
  which lands on whoever next edits `docs/rules/`, because `bun nv session --wrap` runs both for them.
  Reword the sentence so the token sits on one line rather than breaking one across a wrap.
  [until: gone tools/nv/cmd/rules.ts:rules-py:examples]
