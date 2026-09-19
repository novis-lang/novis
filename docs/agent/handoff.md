# Handoff

## State

Goal `lang:types`: every feature in `docs/reference/lang/20-types.md` owes the five artefacts of
`rule:testing/four-proofs`, and the chapter is the whole file set — 18 features, one section each.

Eleven are complete: the nine that were, plus `literals` and `widening-without-as`. Seven still owe;
`python tools/dossier.py --owed --group lang:types` is the list. Nothing is blocked.

`widening-without-as`'s proof found a silent wrong-number bug, and it is **recorded rather than
fixed**: an `array<float>` literal stores an integer element's bits unconverted, so `[4, 2.5]` reads
back `4.9406564584125E-324` for the `4`. It is item 19 of `crates/nvs-ir/src/lib.rs` § *Known gaps*
(owner M10), which carries the analysis and both halves of the fix, and
`docs/examples/lang/types/widening-without-as/03-a-price-list-that-mixes-both.nvs` is the proof
marked `dossier: known-gap`. Its `.out` is hand-written and holds the right answer, so re-blessing
that file hides the bug rather than fixing it. The object-literal half is a representation question
at the shape-assignment boundary, which is why the finding was recorded and not closed in the slice
that found it.

`docs/agent/loop-goal.toml`'s `[context] rules` and its copy at
`docs/agent/goals/dossier/80-lang-types.toml` now carry the next group's chapter rules; keep swapping
them per group rather than naming the chapter's whole `types/` set, which is 47 fragments.

## Next group

**Stage 2: the dossier** — one file set: `docs/reference/lang/20-types.md` and the four proof trees
under `docs/examples/lang/types/`, `tests/hostile/lang/types/`, `benches/members/lang/types/` and
`tests/conformance/`. One slice is one feature with all five artefacts.

- [ ] **`lang:types/the-conversion-operator-as`** — owes all five. `rule:types/conversion`,
      `rule:expressions/nullable-conversion-availability` and
      `rule:expressions/conversion-keeps-qualifiers` specify it; the table there is closed, so a
      count over its rows is the case shape. `docs/reference/lang/20-types.md:526`
- [ ] **`lang:types/narrowing`** — owes all five. `rule:types/narrowing` names exactly four
      spellings, which is a sweep to assert by counting. `docs/reference/lang/20-types.md:659`
- [ ] **`lang:types/truthiness`** — owes all five. `rule:expressions/truthy-positions` names six
      positions and `rule:enums/truthiness` the enum row.
      `docs/reference/lang/20-types.md:735`

## Backlog

- `lang:types/parameters`, `properties-and-constants`, `qualifiers-tainted-and-secret` and
  `what-does-not-exist` still owe all five — `python tools/dossier.py --owed --group lang:types`.
- Gap 19 in `crates/nvs-ir/src/lib.rs` is a correctness bug, not a missing feature: it is owned by
  M10 today, and it may deserve a goal on the chain instead — the user's call.
- `docs/reference/lang/20-types.md:408` lists `\0` through `\777` as octal escapes without saying
  that the assembled string must still be valid UTF-8; `E0431` refuses `"\777"` alone.
