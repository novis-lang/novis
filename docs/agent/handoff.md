# Handoff

## State

**Goal `finish-response` is met — `python tools/loop.py --goal-only` answers `GOAL REACHED: every
acceptance check passes`, and `python tools/verify.py` is 11 of 11 green.** Every stage of the goal is
closed: the ending exists, it unwinds through every `finally`, no `catch` arm admits it, the report
names it, and the spec and the rulebook say so.

**Stage 3's unwind is now asserted in the backend** — six tests in
`crates/nvs-codegen/tests/throwing.rs:327` onward, written against the `THROWS` fixture's shape and a
`FINISHES` sibling whose three frames each hold a `finally` of their own. The lowering needed nothing:
`nvs_ir::lower`'s `lower_finish` already seals the block with a `Terminator::Throw` of the marker, so
what landed is the unwind observed rather than built.

**Stage 5's last red check was a name, not missing work.** `spec_registry_coverage` is a test *file*
and `cargo-named` looks for test *functions*, so it could only ever read "did not run" — the same
correction the goal's own stage 1 floor copy already carried. It now names
`every_part_two_spec_member_is_registered` and
`every_registry_rows_names_are_the_specs_signature_column`, and the spec row those walks read carries
`finish(): void` and the fourth `ExitReason`.

## Next group

**The chain's next goal `markup-literal`, whose own starting handoff is already on disk** — one file
set, `crates/nvs-syntax/`. A goal switch installs that handoff over this one, so these two items are
what to take if the switch has not happened yet.

- [ ] **Stage 2's delimiter and closer** — `crates/nvs-syntax/src/token.rs`, beside `DoubleQuoteOpen`
      and `ComplexInterpClose`: `` html` `` opens and `` ` `` closes, the mode between them being the
      double-quoted one with those two swapped in. `rule:core-classes/html-literal`, and
      `docs/agent/goals/44-markup-literal.handoff.md:21` is the group in full.
- [ ] **Stage 2's unterminated arm** — `crates/nvs-syntax/src/lexer.rs:210-232`, one more `Mode` row,
      so an unterminated literal reports `E0002` at the delimiter that opened it. No new diagnostic
      code; `rule:core-classes/html-literal` and ADR 0169 § *Diagnostics* are why.

## Backlog

- Nothing of `finish-response` is outstanding; `docs/agent/goals/43-finish-response.toml` is the
  acceptance list a switch carries forward as the next goal's floor.
- **Nothing measures what spec §§ 13–20 still owe** — `docs/agent/carried-gaps.md:280`.
