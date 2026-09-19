# Handoff

## State

Goal `lang:types`: every feature in `docs/reference/lang/20-types.md` owes the five artefacts of
`rule:testing/four-proofs`, and the chapter is the whole file set — 18 features, one section each.

Landed and green: `lang:types/array-t` and `lang:types/callable-classes-object-shapes`, each with a
description, three examples, two `.nvst` cases, one attack and a measured figure in
`docs/perf/members.ndjson`. 16 features still owe; `python tools/dossier.py --owed --group lang:types`
is the list. Nothing is blocked, and no proof found a bug it had to record.

`docs/agent/loop-goal.toml`'s `[context] rules` now names the chapter's own rules — it had only the
testing ones, so both slices peeked `types/arrays`, `types/callable-is-a-closure`,
`types/class-reference-sites` and `types/shape-type` by hand. The five added are the ones the next
group reads; a later group adds its own rather than the chapter's whole `types/` set, which is 47
fragments and would double the pack.

## Next group

One slice is one feature with all five artefacts. These three are the next sections in file order,
so they share the chapter and the proof trees under `docs/examples/lang/types/`,
`tests/hostile/lang/types/` and `benches/members/lang/types/`.

Run `target/debug/nvs.exe fmt` over the attack before verifying: `tests/hostile/` is in the
formatter's identity corpus and `docs/examples/` and `benches/members/` are not, which is
`docs/agent/playbook.md:5745` and `:5863`, and it cost this session a whole verification run.

- [ ] **`lang:types/every-binding-has-a-type`** — owes examples, hostile, perf, tests.
      `rule:types/declaration` and `rule:types/var-inference` specify it.
      `docs/reference/lang/20-types.md:9`
- [ ] **`lang:types/numbers-bool-int-uint-float-decimal`** — owes examples, hostile, tests; the bench
      is already on disk and measured. `rule:types/arithmetic`, `rule:types/uint` and
      `rule:types/decimal` specify it. `docs/reference/lang/20-types.md:68`
- [ ] **`lang:types/text-string-and-bytes`** — owes examples, hostile, perf, tests.
      `rule:types/string-is-utf8` and `rule:types/bytes` specify it.
      `docs/reference/lang/20-types.md:119`

## Backlog

- `rule:types/arrays` calls `as array<int|string>` the spelling that restamps an array, and the
  binary refuses a union element type as an `as` target with `E0711` — pinned by
  `tests/conformance/lang/an-array-target-whose-element-type-no-tag-decides-is-refused.nvst`, so the
  rule's example is what is stale. `docs/rules/types/arrays.md:43`.
- A call through a `callable` heap-allocates 32 bytes for the `mixed` it answers, once per call —
  `docs/perf/members.ndjson`, `lang:types/callable-classes-object-shapes`, now declared
  `// bench: allocations 1` so a regression shows. An opportunity on the request path, not a fault.
- `array<never>` written as a binding's declared type does not widen to `array<string>`, where the
  empty literal does; nobody writes it, and `docs/rules/types/arrays.md:47` speaks of the literal
  only. Not worth a case unless a later feature needs one.
- 13 features of goal `lang:types` after the three named above; `--owed --group lang:types` ranks them.
