# Handoff

## State

Goal `lang:types`: every feature in `docs/reference/lang/20-types.md` owes the five artefacts of
`rule:testing/four-proofs`, and the chapter is the whole file set — 18 features, one section each.

Landed and green: `array-t`, `callable-classes-object-shapes`, `every-binding-has-a-type` and
`numbers-bool-int-uint-float-decimal`. 14 features still owe; `python tools/dossier.py --owed
--group lang:types` is the list. Nothing is blocked, and no proof found a bug it had to record.

`numbers-bool-int-uint-float-decimal`'s two cases are `covers:` markers added to cases the corpus
already held — `decimal-arithmetic-is-exact-and-keeps-its-scale` and
`an-integer-overflow-throws-rather-than-wrapping`. `rule:testing/proof-attribution` says that is
the whole edit for a language feature, and a new case over the same claims would have been another
row of a shape `docs/agent/conventions.md` § *A `.nvst` test case* names as the thing not to write.
Check what the corpus pins before drafting a pair for a feature whose behaviour is old.

`docs/agent/loop-goal.toml`'s `[context] rules` and its copy under `docs/agent/goals/dossier/` now
carry the next group's chapter rules instead of the landed group's; keep swapping them per group
rather than naming the chapter's whole `types/` set, which is 47 fragments.

## Next group

**Stage 2: the dossier** — one file set: `docs/reference/lang/20-types.md` and the four proof trees
under `docs/examples/lang/types/`, `tests/hostile/lang/types/`, `benches/members/lang/types/` and
`tests/conformance/`. One slice is one feature with all five artefacts.

- [ ] **`lang:types/text-string-and-bytes`** — owes examples, hostile, perf, tests.
      `rule:types/string-is-utf8` and `rule:types/bytes` specify it.
      `docs/reference/lang/20-types.md:119`
- [ ] **`lang:types/mixed`** — owes all five. `rule:types/unions-and-mixed` and
      `rule:types/mixed-subscript` specify it. `docs/reference/lang/20-types.md:230`
- [ ] **`lang:types/nullable-union-literal-and-enum-case-types`** — owes all five.
      `rule:types/literal-types` and `rule:types/unions-and-mixed` specify it.
      `docs/reference/lang/20-types.md:265`

Run `target/debug/nvs.exe fmt` over a new attack before verifying: `tests/hostile/` is in the
formatter's identity corpus and `docs/examples/` and `benches/members/` are not, which is
`docs/agent/playbook.md:5745` and `:5863`.

## Backlog

- A `decimal` class constant is refused at its read with `E0792`, while
  `docs/reference/lang/20-types.md` § *Properties and constants* says a constant's value is "a
  scalar literal" without naming the exception; `crates/nvs-types/src/defaults.rs`'s module doc
  owns the missing `ConstArg` variant.
- `about.md` is written for every feature this goal lands, and `--owed` does not count it yet; the
  owed check is deferred until the dossier goals are done.
