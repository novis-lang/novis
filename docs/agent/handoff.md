# Handoff

## State

**Goal `m4-refusals` is met and its floor is green again.** `python tools/holes.py` reports 0 refusal
sites across both crates, `CEILING` is `0`, `ALLOWLIST` is empty and `docs/agent/carried-refusals.md`
holds no entry. The acceptance check that held the previous done-claim —
`python tools/chain.py --check` — now exits 0 with 2 notes and no problem.

- **A `for` header whose condition clause is a comma list lowers.**
  `crates/nvs-ir/src/lower/control.rs:451` evaluates every expression in the clause into the header
  as the statement it is and decides on the last, which is PHP's rule; the phi seeding already walked
  the whole clause, so a local a discarded expression writes carries across the back edge. Checked
  against `D:\srv\php\php.exe` on the same two programs, output for output, including the `continue`
  path and the trip that fails the test.
- **`crates/nvs-ir/src/lib.rs`'s gap 1 is deleted and leaves a hole**, per that section's own
  preamble. Its `switch`/`match` half was already a decision `lower_switch`'s doc comment owns, and
  the `for` half is the shape above.
- `docs/reference/lang/40-statements.md` now states the last-decides rule, and `docs/novis.md` is
  regenerated from it.
- `python tools/verify.py` green over the slice; conformance is 1895.

## Next group

**The next goal's, not this one's** — `m4-refusals` has no stage left. This item is a finding the
previous session made and neither session could take: the goal's § *Standing decisions* assigns every
other gap in `crates/nvs-ir/src/lib.rs` to goal `unowned-closures`, and it is not a refusal the gate
can see.

- [ ] **Gap 3 names nothing outstanding any more** — `crates/nvs-ir/src/lib.rs:219`. Its closing
      claim was the tagged-arithmetic panic, which is gone; deleting the whole entry needs a sweep of
      every `Ty::Tagged` reading rather than the four rows two sessions have now run by hand, and the
      § *Known gaps* preamble says a closed gap is deleted and leaves a hole rather than renumbering.
- [ ] **`carried-gaps.md`'s `nvs-ir` entry lists panics that no longer exist** —
      `docs/agent/carried-gaps.md:182`. It names `<=>` and `**` having no `ir::BinOp` row and a
      ternary having no recorded result type to widen to, while `holes.py` reports 0 sites; the same
      `Ty::Tagged` sweep settles which sentences of it survive.

## Backlog

- `holes.py`'s `REFUSAL` reads a phrasing, not a kind — the source-declared kind the crate's
  § *Known gaps* preamble describes is what would end this class (`tools/holes.py:96`).
- `crates/nvs-ir/tests/refusals.rs`'s gate is only as wide as that recognizer; its module doc says so.
- `docs/agent/goals/55-m5-proofs.toml` and `57-m7-server-surface.toml` hold prose against rules no
  `rules` entry reaches — `chain.py --check`'s two standing notes.
