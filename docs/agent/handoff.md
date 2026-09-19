# Handoff

## State

Goal `lang:types` is **met**: all 18 features of `docs/reference/lang/20-types.md` carry the five
artefacts of `rule:testing/four-proofs`, and `python tools/dossier.py --verify --group lang:types`
reports nothing owed with 0 failed on both suites. `qualifiers-tainted-and-secret` and
`what-does-not-exist` landed this session. Nothing is blocked.

Two things this session changed outside the proofs. The chapter claimed no member removes `secret`;
`Core\Secret::reveal` has always existed (`rule:core-classes/secret-reveal`), so the chapter now says
so. And a guard clause ending in `break` or `continue` did not narrow what followed it, which
`crates/nvs-types/src/locals.rs:827`'s doc comment now explains and a conformance case pins.

**A chapter edit re-prices the whole group's perf figures.** `impl_file` for a `lang:` feature is the
reference chapter itself, so touching `docs/reference/lang/20-types.md` stales all 18 records at
once; `python tools/dossier.py --record-perf --group lang:types` re-measures the lot in about four
seconds. It was re-run after this session's edit.

`docs/agent/goals/dossier/80-lang-types.toml`'s `[context] rules` never printed the rules this goal's
own items cite — `security/sink-predicate`, `security/every-grammar-is-a-sink`,
`security/unclassified-parameter-refuses-tainted`, `core-classes/secret-reveal` and
`types/narrowing` all cost a `peek.py` call each.

## Next group

**Stage 2: the dossier, goal `lang:types` met** — one file set: `docs/reference/lang/20-types.md` and
the four proof trees under `docs/examples/lang/types/`, `tests/hostile/lang/types/`,
`benches/members/lang/types/` and `tests/conformance/`. The chain moves on to goal
`lang-expressions`, whose own handoff replaces this one; these two are what is left here if it does
not.

- [ ] **Re-measure the group after the next chapter edit** — every `lang:types` record's `impl_file`
      is the chapter, so one edit stales all 18 and
      `python tools/dossier.py --record-perf --group lang:types` is the whole chore.
      `docs/reference/lang/20-types.md:834`
- [ ] **`lang:types/widening-without-as`'s third example is still marked `dossier: known-gap`** — a
      whole number written straight into an `array<float>` keeps its integer bits, item 19 of
      `crates/nvs-ir/src/lib.rs` § *Known gaps*. Removing the marker is part of that fix.
      `docs/examples/lang/types/widening-without-as/03-a-price-list-that-mixes-both.nvs:1`

## Backlog

- `Core\Regex::compile`'s refusal message carries the whole pattern, so a 2 MiB pattern makes a
  3.6 MiB log line; `crates/nvs-stdlib/src/regex.rs` has no `# Known gaps` section and `owners.py`
  refuses `unowned`, so the owner is a call only the user can make.
- `rule:statements/an-inout-argument-is-a-local` says a property is refused at an `inout` argument;
  the binary accepts a plain one, writes it back, and refuses only an element and a *hooked*
  property (`E0439`, `crates/nvs-types/src/expr/assign.rs`). The chapter agrees with the binary.
  One of the two is wrong and picking is an ADR.
- `rule:statements/inout-is-the-by-reference-spelling` shows `[inout int $a, …] = $pair;` as legal;
  the binary refuses a destructuring leaf with `E0483`, pinned by two conformance cases. ADR 0107
  § *Context* named the leaf as one of the three surviving positions.
- An enum case that reached a `mixed` is falsy when its backing integer is `0` —
  `crates/nvs-runtime/src/lib.rs` § *Known gaps* item 9, owner M10.
