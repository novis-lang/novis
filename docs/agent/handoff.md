# Handoff

## State

Goal `lang-statements`: two of its thirteen features now carry complete feature proofs —
`python tools/dossier.py --id 'lang:statements/break-and-continue'` and the same for
`lang:statements/catch-as-an-expression` both print `complete.`. Eleven are left, all anchored in
`docs/reference/lang/40-statements.md`.

Both features already had two conformance cases on disk that nobody had attributed, so the tests
column was closed by a `// covers:` marker rather than by a new case — check that first for every
remaining item, because `rule:testing/proof-attribution` credits a language feature by marker alone.
Nothing is blocked.

## Next group

**One file set: `docs/reference/lang/40-statements.md` plus the four proof trees under
`lang/statements/`.** One slice is one feature with all its feature proofs
(`rule:testing/feature-proofs`): `about.md`, three examples with blessed `.out` files, one attack,
one bench, and two tests carrying a `covers:` marker.

- [ ] **`lang:statements/echo-print-unset-exit-yield`** — owes examples, hostile, perf, tests.
      `unset` of a local or a property is a compile error and `exit` ends the program, so the attack
      needs `// hostile: ends-early`. `docs/reference/lang/40-statements.md:445`
- [ ] **`lang:statements/throw`** — owes examples, hostile, perf, tests. An uncaught `throw` ends the
      program with status 1. `docs/reference/lang/40-statements.md:432`
- [ ] **`lang:statements/try-catch-finally`** — owes examples, hostile, perf, tests. The block form
      the expression `catch` lowers to; its proofs may not repeat that feature's.
      `docs/reference/lang/40-statements.md:294`

## Backlog

- The loop family — `for`, `foreach`, `while-and-do-while` — is the natural group after this one; the
  three share one understanding of an iteration. `docs/agent/loop-goal.md:29`
- `E0475`'s label reads `2 enclosing loop/`switch` here` when two loops enclose the level: the count
  is plural and the nouns are not. Only the depth-1 wording is pinned today, in
  `tests/conformance/lang/a-break-level-that-names-no-target-is-a-diagnostic.nvst`.
  `crates/nvs-types/src/locals.rs:960`
- Building a string by repeated `.` in a loop is the dominant cost in a hostile case that wants a
  huge value: 400k appends took ~11s of a 30s budget under the debug binary, 200k took ~5s.
