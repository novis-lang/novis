# Handoff

## State

Goal `lang:expressions` is ten features in: the eight that landed before, plus
`comparison-and-equality` and `logical-operators-and-truth`, each now carrying all five artefacts.
Six of the chapter's sixteen still owe theirs — `python tools/dossier.py --owed --group
'lang:expressions'` is the list.

The reject proof of `comparison-and-equality` found `E0466`'s help naming a cast for rows where no
cast exists: an enum against a number was told to write `$s == ($n as string)`, and so were two
unrelated classes, for which no conversion exists at all. `disjoint_help` in
`crates/nvs-types/src/expr/operators.rs:449` now picks the sentence from the two operands' domains,
and the reject case pins all four wordings.

Two facts the binary settled against what a proof was written to assume. Narrowing does not reach
the right operand of a `&&` (`rule:types/narrowing` is branch-local), so the guard idiom
`$x != null && f($x)` does not compile — the playbook bullet has the replacement. And a `string`
read out of an `array<string>` costs two allocations per read, so a bench that wants
`allocations 0` holds its text in locals; that cost belongs to `arrays-in-expressions`, not to
either feature here, and is not otherwise written down.

`python tools/verify.py` is green. `nvs fmt` rewrites a non-interpolating `"…"` to `'…'`, and the
`nvs-fmt` identity corpus covers `tests/hostile/` — so a new attack file is formatted before the
verification, or the corpus test fails.

## Next group

One slice is one feature with all five proofs. The three below are the chapter's next three
features in chapter order, and they share the file set the ten landed ones used:
`docs/reference/lang/30-expressions.md` plus the four proof trees under
`docs/examples/lang/expressions/`, `tests/hostile/lang/expressions/`,
`benches/members/lang/expressions/` and `tests/conformance/`.

**Stage 2: the dossier** — one file set, named above.

- [ ] **`lang:expressions/string-operators`** — owes all five. `.` joins and `.=` appends, either
      side may be a `string`, a number, a `bool`, `null` or a `Stringable`, and `bytes`, an array
      and an enum case are refused — so the two cases split into one sweep over the operand table
      and one reject case over the three that have no string form.
      `rule:types/string-is-utf8`. `docs/reference/lang/30-expressions.md:283`
- [ ] **`lang:expressions/assignment`** — owes examples, perf and tests; the dossier asks for no
      attack. `=` is an expression, every `op=` is `$x = $x op e` with the target read once, and a
      compound form cannot retype its target — `$n /= 2` on an `int` is refused because `/` answers
      `int|float`. `rule:types/declaration`. `docs/reference/lang/30-expressions.md:346`
- [ ] **`lang:expressions/match`** — owes all five. Arms compare with `==`, so a disjoint condition
      is the same `E0466` the slice above pinned; `match (true)` is the ordered-condition form, and
      no arm plus no `default` throws a `LogicError`.
      `rule:expressions/switch-match-equality`, `rule:expressions/mixed-equality`.
      `docs/reference/lang/30-expressions.md:368`

## Backlog

- `lang:expressions` still owes `is-new-clone-throw-print-exit-isset-empty`, `calls` and `closures`
  after the group above — `python tools/dossier.py --owed --group 'lang:expressions'`.
- A `string` read out of an `array<string>` allocates twice per read; nothing owns that figure yet,
  and `benches/members/lang/expressions/arrays-in-expressions.nvs` is where it would be measured.
- `E0715`'s help for a `bytes` and an `array<T>` operand is the same generic sentence; `string` has
  a specific one (`Core\Str::compare`) and the other two rows could too.
