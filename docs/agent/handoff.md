# Handoff

## State

**Goal `m4-refusals` is met: `python tools/holes.py` reports 0 refusal sites across both crates and
`CEILING` is `0`.** Every shape `nvs-ir` type-checks either lowers or is a `lower::guarded_by!`
naming the diagnostic that refuses it where it is written, with a conformance case expecting that
code; `ALLOWLIST` is still empty and `docs/agent/carried-refusals.md` now holds no entry at all.

- **`is` against a whole enum was a live lowering gap, not a message to reword.** The handoff's claim
  that stage 7 had closed the shape was wrong: `$m is Rank` panicked at
  `crates/nvs-ir/src/lower/expr.rs:4985`. It is now the disjunction of the enum's cases — one
  `TestShape::Literal` each, over `convert::sorted_enum_cases`, which `as EnumName` walks too — so the
  whole-enum row answers exactly what its own case rows answer.
- **The `is` backstop is an engine invariant now, naming each guarantee**: a qualifier (`E0813`),
  `void`/`never` (`E0811`), `mixed` (settled at the checker), a compiler-owned type no source spells,
  and a signature `nvs_types::check` recorded no marker for.
- **Two gap claims in `crates/nvs-ir/src/lib.rs` were disproved by running them** — `C::$p = v` writes,
  and every arithmetic and bitwise row over two `mixed` operands lowers through the
  `Helper::ValueAdd` family. Both sentences are gone; gap 3 may now be empty altogether, which is its
  owner's call and not this goal's.
- **One real refusal is invisible to the gate** — the playbook bullet added this session owns it, and
  it is lib.rs gap 1's other half, which stays open and `unowned`.
- `python tools/verify.py` green over the group; conformance is 1894.

## Next group

**The next goal's, not this one's** — `m4-refusals` has no stage left. Both items below are findings
this session made and could not take: the goal's § *Standing decisions* assigns every other gap in
`crates/nvs-ir/src/lib.rs` to goal `unowned-closures`, and neither is a refusal the gate can see.

- [ ] **A `for` header whose condition clause is a comma list still refuses** —
      `crates/nvs-ir/src/lower/control.rs:460`. PHP evaluates every expression and decides on the
      last, which is what the panic's own message says it will not do; lib.rs gap 1
      (`crates/nvs-ir/src/lib.rs:209`) is its index and `rule:iteration/for-init-clause` the
      neighbouring rule. `holes.py` does not count this site, so closing it moves no number.
- [ ] **Gap 3 names nothing outstanding any more** — `crates/nvs-ir/src/lib.rs:234`. Its closing claim
      was the tagged-arithmetic panic, which is gone; deleting the whole entry needs a sweep of every
      `Ty::Tagged` reading rather than the four this session ran, and the § *Known gaps* preamble says
      a closed gap is deleted and leaves a hole rather than renumbering.

## Backlog

- `holes.py`'s `REFUSAL` reads a phrasing, not a kind — the source-declared kind the crate's
  § *Known gaps* preamble describes is what would end this class (`tools/holes.py:96`).
- `crates/nvs-ir/tests/refusals.rs`'s gate is only as wide as that recognizer; its module doc says so.
