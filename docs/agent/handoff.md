# Handoff

## State

Goal `lang:expressions` is twelve features in: `string-operators` and `assignment` now carry all
five artefacts each. Four of the chapter's sixteen still owe theirs — `python tools/dossier.py
--owed --group 'lang:expressions'` is the list.

The `assignment` bench found a latency gap and it is recorded rather than fixed: a `uint` array
subscript is rendered to a decimal string key on every access
(`crates/nvs-ir/src/lower/expr.rs:2374`, `Helper::UintToString`), so `$a[$i] += $v` with a `uint`
`$i` costs four allocations a round where the same index declared `int` costs none. That is gap 21
in `crates/nvs-ir/src/lib.rs`, owner M12, and the bench carries the `dossier: known-gap` marker —
removing the marker is part of whatever fix lands.

An array type takes one parameter only; the playbook bullet has the spelling. `python
tools/verify.py` is green.

## Next group

One slice is one feature with all five proofs. The three below are the chapter's next three
features in chapter order, and they share the file set the twelve landed ones used:
`docs/reference/lang/30-expressions.md` plus the four proof trees under
`docs/examples/lang/expressions/`, `tests/hostile/lang/expressions/`,
`benches/members/lang/expressions/` and `tests/conformance/`.

**Stage 2: the dossier** — one file set, named above.

- [ ] **`lang:expressions/match`** — owes all five. Arms compare with `==`, so a disjoint condition
      is a compile error and the reject case is that plus a `match` with no arms; `match (true)` is
      the ordered-condition form, and no arm matching with no `default` throws a `LogicError`, which
      is the hostile case's lever. `rule:expressions/disjoint-comparison-refused`.
      `docs/reference/lang/30-expressions.md:368`
- [ ] **`lang:expressions/is-new-clone-throw-print-exit-isset-empty`** — owes all five. Eight
      keyword expressions in one feature, so the two cases split into one sweep over `is`/`isset`/
      `empty` against every storage shape and one over `new`/`clone`/`throw`/`print`/`exit` as
      expressions. `rule:php-migration/absent-storage-is-never-a-zero-value`.
      `docs/reference/lang/30-expressions.md:394`
- [ ] **`lang:expressions/calls`** — owes all five. Named arguments, spread, and what a call's
      target may be; `rule:types/callable-is-a-closure` is the refusal half, and the hostile case is
      recursion depth plus an argument list sized past what a frame holds.
      `docs/reference/lang/30-expressions.md:450`

## Backlog

- `python tools/dossier.py --gaps` walks examples and hostile cases only, so a `known-gap` marker on
  a *bench* is visible only in `--record-perf` output — `tools/dossier.py`.
- `lang:expressions/closures` is the chapter's last owed feature after the three above —
  `docs/reference/lang/30-expressions.md:535`.
- A `uint` array subscript's key rendering is gap 21 in `crates/nvs-ir/src/lib.rs`; it is owned by
  M12 and is not this goal's to close.
