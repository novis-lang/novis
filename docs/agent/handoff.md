# Handoff

## State

Goal `lang:types`: every feature in `docs/reference/lang/20-types.md` owes the five artefacts of
`rule:testing/four-proofs`, and the chapter is the whole file set — 18 features, one section each.

Sixteen are complete. `truthiness`, `parameters` and `properties-and-constants` landed this session;
`qualifiers-tainted-and-secret` and `what-does-not-exist` are what is left, and
`python tools/dossier.py --owed --group lang:types` is the list. Nothing is blocked.

**A chapter edit re-prices the whole group's perf figures.** `impl_file` for a `lang:` feature is
the reference chapter itself, so touching `docs/reference/lang/20-types.md` makes all 18 records
stale at once. `python tools/dossier.py --record-perf --group lang:types` re-measures the lot in
about four seconds, so this is a chore rather than a reason not to edit the chapter — but it has to
be re-run after the edit, not before.

Two findings this session, both doc-vs-binary and both in `## Backlog` below; one is recorded as
`crates/nvs-runtime/src/lib.rs` gap 9. Still open from before:
`docs/examples/lang/types/widening-without-as/03-a-price-list-that-mixes-both.nvs` is marked
`dossier: known-gap` for item 19 of `crates/nvs-ir/src/lib.rs` § *Known gaps*.

## Next group

**Stage 2: the dossier** — one file set: `docs/reference/lang/20-types.md` and the four proof trees
under `docs/examples/lang/types/`, `tests/hostile/lang/types/`, `benches/members/lang/types/` and
`tests/conformance/`. One slice is one feature with all five artefacts; these two close the goal.

- [ ] **`lang:types/qualifiers-tainted-and-secret`** — owes all five. `rule:security/sink-predicate`
      says what a sink is and `rule:security/unclassified-parameter-refuses-tainted` what a `Core`
      parameter does with an unclassified one; `rule:security/log-is-not-a-sink` and
      `rule:security/capture-answers-the-carrier` are the two a reader gets wrong, and
      `rule:security/every-grammar-is-a-sink` is what an attack aims at. A count over the sinks is
      the case shape. `docs/reference/lang/20-types.md:834`
- [ ] **`lang:types/what-does-not-exist`** — owes all five, and it is the chapter's list of
      refusals, so the artefacts do not fall out the usual way: a hostile case whose program does
      not compile is a **failure**, not a pass (`rule:testing/hostile-case-contract`), so the attack
      has to be a program that runs while trying every replacement spelling at once. The examples
      are the replacements — `Core\Str::length` for `strlen`, `Core\Env::EOL` for `PHP_EOL`
      (`rule:classes/no-free-functions-or-constants`) — and the refusals themselves belong in a
      `tests/conformance/reject/` case. `docs/reference/lang/20-types.md:952`

## Backlog

- `rule:statements/an-inout-argument-is-a-local` says a property is refused at an `inout` argument;
  the binary accepts a plain one, writes it back, and refuses only an element and a *hooked*
  property (`E0439`, `crates/nvs-types/src/expr/assign.rs`). The chapter agrees with the binary.
  One of the two is wrong and picking is an ADR.
- `rule:statements/inout-is-the-by-reference-spelling` shows `[inout int $a, …] = $pair;` as legal;
  the binary refuses a destructuring leaf with `E0483`, pinned by two conformance cases. ADR 0107
  § *Context* named the leaf as one of the three surviving positions.
- An enum case that reached a `mixed` is falsy when its backing integer is `0` —
  `crates/nvs-runtime/src/lib.rs` § *Known gaps* item 9, owner M10.
- `docs/examples/lang/types/widening-without-as/03-a-price-list-that-mixes-both.nvs` carries item 19
  of `crates/nvs-ir/src/lib.rs` § *Known gaps*.
