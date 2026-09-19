# Handoff

## State

Goal `lang-statements`: eight of its thirteen features carry complete feature proofs.
`python tools/dossier.py --id '<feature>'` prints `complete.` for `break-and-continue`,
`catch-as-an-expression`, `echo-print-unset-exit-yield`, `throw`, `try-catch-finally`,
`if-elseif-else`, `foreach` and `while-and-do-while`. Five are left, all anchored in
`docs/reference/lang/40-statements.md`, and `python tools/dossier.py --owed --group lang:statements`
lists them. The group's acceptance check stays red until all thirteen are done; it names the
earliest owed feature, which is the ordinary in-progress state and not a regression.

The tests column keeps closing by a `// covers:` marker on a case already on disk that nobody had
attributed (`rule:testing/proof-attribution`) — nine cases so far, under `tests/conformance/error/`,
`tests/conformance/lang/` and `tests/conformance/array/`. Look for one before writing a new case.
Nothing is blocked.

## Next group

**One file set: `docs/reference/lang/40-statements.md` plus the five proof trees under
`lang/statements/`.** One slice is one feature with all its feature proofs
(`rule:testing/feature-proofs`): `about.md`, three examples with blessed `.out` files, one attack,
one bench, and two tests carrying a `covers:` marker.

- [ ] **`lang:statements/for`** — owes examples, hostile, perf, tests. The three-part header, a
      comma list in any of the three parts, and the condition deciding on its last expression.
      `tests/conformance/lang/a-for-loop-runs-init-condition-and-step.nvst` and
      `a-for-condition-that-is-a-comma-list-decides-on-its-last-expression.nvst` are both on disk
      and unattributed. `docs/reference/lang/40-statements.md:100`
- [ ] **`lang:statements/switch`** — owes examples, hostile, perf, tests. The `case`/`default`
      arms, what Novis does about fallthrough, and `break` out of an arm.
      `docs/reference/lang/40-statements.md:177`
- [ ] **`lang:statements/return`** — owes examples, hostile, perf, tests. A bare `return` in a
      `void` method, returning out of a loop, and the value's declared type.
      `docs/reference/lang/40-statements.md:265`

## Backlog

- `lang:statements/expression-statements-blocks-and-declarations` still owes every proof — `python tools/dossier.py --owed --group lang:statements`.
- `lang:statements/statement-forms-that-do-not-parse` owes every proof, and an example there cannot run: decide what an example of a form that does not parse is before taking it.
- `foreach` over an array measures 115.9 ns per element visit against 4.7 ns for an `if` arm — `docs/perf/members.ndjson`, not investigated.
