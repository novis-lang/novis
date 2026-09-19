# Handoff

## State

Goal `lang-statements` is complete. All thirteen features carry their feature proofs, and `python
tools/dossier.py --verify --group lang:statements` prints `nothing owed`, `0 failed`, `0 failed` —
the goal's own acceptance check. The last two features landed this session:
`expression-statements-blocks-and-declarations` and `statement-forms-that-do-not-parse`.
`docs/perf/members.ndjson` carries a current figure for both new benches.

The tests column closed four more times by a `covers:` marker on a case already on disk
(`rule:testing/proof-attribution`) rather than by a new case. Two of those are `--EXPECTF-ERROR--`
reject cases, where the marker has to be appended at the *end* of the `--FILE--` block; the playbook
bullet under *Writing a test case* says why. Nothing is blocked.

## Next group

**Goal `lang-classes` — one file set: `docs/reference/lang/50-classes.md` plus the proof trees under
`lang/classes/`.** The goal switch installs that goal's own handoff over this one; these are its
first three features in file order, so the anchors are here if the switch is delayed. One slice is
one feature with all its feature proofs (`rule:testing/feature-proofs`): `about.md`, three examples
with blessed `.out` files, one attack, one bench, and two tests carrying a `covers:` marker.

- [ ] **`lang:classes/declaring-a-class`** — owes examples, hostile, perf, tests.
      `docs/reference/lang/50-classes.md:9`
- [ ] **`lang:classes/constants-and-class`** — owes examples, hostile, perf, tests.
      `docs/reference/lang/50-classes.md:374`
- [ ] **`lang:classes/comparable`** — owes examples, hostile, perf, tests.
      `docs/reference/lang/50-classes.md:1008`

## Backlog

- `declare(strict_types=…)` is refused only as a cascade of four generic parser and name-resolution
  errors, while `goto`, `global`, function-scope `static`, `list()` and `include` each get a named
  `E02xx` naming the replacement. `docs/reference/lang/40-statements.md:479`
- The alternative syntax `if (…): … endif;` has no named diagnostic and no conformance case either;
  it falls out as `expected an expression`. `docs/reference/lang/40-statements.md:480`
