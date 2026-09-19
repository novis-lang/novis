# Handoff

## State

Goal `lang:expressions` is fourteen of sixteen features done: `match` and
`is-new-clone-throw-print-exit-isset-empty` now carry all five artefacts each. Only `calls` and
`closures` still owe theirs — `python tools/dossier.py --owed --group 'lang:expressions'` is the
list, and closing those two closes the goal.

Both new features are attributed partly through existing cases: `covers:` markers were added to
`tests/conformance/lang/a-match-picks-one-arm-and-throws-when-none-does.nvst`,
`tests/conformance/lang/an-arm-less-match-is-a-diagnostic.nvst`,
`tests/conformance/lang/isset-answers-a-null-test-over-every-storage-shape.nvst` and
`tests/conformance/reject/clone-takes-an-object.nvst` rather than rewriting what they already pin,
and each feature also got one new depth case. Adding the marker shifts every line under it, so an
`--EXPECTF-ERROR--` block's `%s:NN` numbers move with it.

No proof found a bug. One measurement did move: the first
`is-new-clone-throw-print-exit-isset-empty` bench read `isset`/`empty` through a string-keyed array
element and measured 61.8 ns/op against 13 statements, four times the per-statement cost of every
other expression bench. Reading the same two tests off a local and a property instead gives
14.1 ns/op, so what the first figure measured was the array key lookup, not this feature. The
lookup cost itself is not recorded as a gap — nothing is known to be wrong with it — but a bench
for `lang:expressions/arrays-in-expressions` would be where to look.

`python tools/verify.py` is green.

## Next group

One slice is one feature with all five proofs. These are the chapter's last two, and they share the
file set the fourteen landed ones used: `docs/reference/lang/30-expressions.md` plus the four proof
trees under `docs/examples/lang/expressions/`, `tests/hostile/lang/expressions/`,
`benches/members/lang/expressions/` and `tests/conformance/`.

**Stage 2: the dossier** — one file set, named above.

- [ ] **`lang:expressions/calls`** — owes all five. Named arguments in any order, one that skips a
      defaulted parameter in the middle, a `...$array` spread into the single variadic, `inout` at
      both ends of the call, and a `callable` invoked through a variable, an element or a
      parenthesised property. Evaluation is in written order, extra arguments to a `callable` are
      dropped and too few throw at the call. `rule:core-api/parameters-are-callable-by-name`.
      `docs/reference/lang/30-expressions.md:450`
- [ ] **`lang:expressions/closures`** — owes all five. `fn` is the only closure literal and the
      only value a `callable` takes; every outer local the body reads is captured **by value when
      the closure is created**, which is the example worth writing and the hostile case's lever
      (capture at size, and `$this` captured inside a method). The reject half is the list the
      section closes: no `use (…)`, no capture by reference, no `static fn`, no `inout` parameter,
      no anonymous `function () {}`, and `Class::m(...)` is not a closure.
      `rule:types/callable-is-a-closure`. `docs/reference/lang/30-expressions.md:535`

## Backlog

- `lang:expressions/arrays-in-expressions` has no bench measuring a string-keyed element read; the
  figure above suggests one would be worth having (`benches/members/README.md`).
- The help on `E0466` names `$s == ($n as string)` whatever the operands are, so a `match` arm
  refused for a disjoint condition is advised to rewrite a comparison it did not write
  (`crates/nvs-diagnostics`, and the wording is pinned by four cases).
