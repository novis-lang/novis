# Handoff

## State

Goal `lang-statements`: four of its thirteen features carry complete feature proofs.
`python tools/dossier.py --id '<feature>'` prints `complete.` for `break-and-continue`,
`catch-as-an-expression`, `echo-print-unset-exit-yield` and `throw`. Nine are left, all anchored in
`docs/reference/lang/40-statements.md`, and `python tools/dossier.py --owed --group lang:statements`
lists them.

The tests column keeps closing by a `// covers:` marker on a case already on disk that nobody had
attributed (`rule:testing/proof-attribution`) — four cases so far, two of them under
`tests/conformance/error/`. Look for one before writing a new case. Nothing is blocked.

## Next group

**One file set: `docs/reference/lang/40-statements.md` plus the four proof trees under
`lang/statements/`.** One slice is one feature with all its feature proofs
(`rule:testing/feature-proofs`): `about.md`, three examples with blessed `.out` files, one attack,
one bench, and two tests carrying a `covers:` marker.

- [ ] **`lang:statements/try-catch-finally`** — owes examples, hostile, perf, tests. The block form,
      a multi-catch written as two clauses, and `finally` on every way out. Two `catch` clauses in
      one function need two variable names, which is `E0406` and not a `catch` rule.
      `docs/reference/lang/40-statements.md:294`
- [ ] **`lang:statements/if-elseif-else`** — owes examples, hostile, perf, tests. `else if` and
      `elseif` are both written, and the `endif` form does not parse.
      `docs/reference/lang/40-statements.md:44`
- [ ] **`lang:statements/foreach`** — owes examples, hostile, perf, tests. The key form, `inout`,
      and what a write during the loop does. `docs/reference/lang/40-statements.md:137`

## Backlog

- `unset($a[$i])` with an `int` subscript costs three allocations, because the key is rendered to
  its decimal — `crates/nvs-ir/src/lower/expr.rs:2406` owns the reasoning and defers the ABI
  widening. The bench uses a string key so its figure prices the statement.
- One raise and one catch across one frame measures 16 allocations and 906 bytes per operation
  (`docs/perf/members.ndjson`, `lang:statements/throw`). Nothing declares a bound for it.
- The three tree READMEs (`docs/examples/`, `tests/hostile/`, `benches/members/`) are ~8k of
  context every dossier session; `[context]` has no field that inlines a body, so peek all three in
  one call rather than one at a time.
