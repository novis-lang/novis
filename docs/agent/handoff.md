# Handoff

## State

**Goal 18. Stage 4's group is closed — all four items landed, and both of the stage's acceptance
checks now name only tests and cases that exist and pass.** Nothing in stage 4 needed designing:
`postAs` and `queryAs` were already implemented, so every item was a test over landed behaviour or a
correction to where a check was filed.

`crates/nvs-stdlib/src/request.rs`'s inline `mod tests` gained the five names the `cargo-named` check
lists, over an **inline shape** rather than a named class — that is what a request call site writes,
and a shape declares no constructor, so the fixture is `define_shape_codec` and not `reading_class`'s
`set_methods`. `n: int` is in the shape on purpose: a form carries text and nothing else, so the
`int` field is the only thing that separates `Reading::Values` from the wire reading `jsonAs` picks.

**The `nvs-suite` check was amended rather than satisfied, and this is the session's one substantive
finding.** All four of its drafted `.nvst` paths were already pinned by three cases the corpus had
taken other names for — `a-request-parameter-set-becomes-the-shape-it-was-asked-for.nvst` alone
carries three of the four claims (both members, the whole set, and the unnamed key left behind).
Each was run and passes.

**`an_unqualified_shape_at_a_request_call_site_is_diagnosed_naming_the_shape` was dropped from the
`-p nvs-stdlib` list**, not written: the diagnosis is a refusal the checker writes, and
`crates/nvs-stdlib/Cargo.toml` names neither `nvs-types` nor `nvs-hir`. The claim is pinned by
`tests/conformance/reject/an-unqualified-shape-at-a-request-decode-site-is-refused.nvst`, which now
sits in the stage's `nvs-suite` check.

**Stage 5 is further along than its checks suggest, and two of its three are already green.**
`examples/input-shapes.nvs` and `examples/input-shapes.nvsr` are both on disk and
`nvs run --request examples/input-shapes.nvsr examples/input-shapes.nvs` prints all six `want`
lines exactly; `python tools/reference.py --check` passes. `docs/reference/core/Request.md` and
`docs/reference/core/Arr.md` already carry the members. What is unconfirmed is only the stage-5
`cargo-named` gate.

## Next group

**Stage 5: the proofs — confirm what is left, then close it** — one file set:
`crates/nvs-stdlib/tests/` and `docs/agent/loop-goal.toml`'s stage-5 block. `rule:core-api/shape-rules`
R15 and `rule:core-api/reference-card` are the specification; the goal's stage 5 prose is
`docs/agent/loop-goal.md:"## Stage 5"`. **Verify before writing:** two of the three checks were run
this session and pass, so the group is smaller than the stage reads.

- [ ] **The five roster gates, run first and closed only where one fails** —
      `crates/nvs-stdlib/tests/spec_registry_coverage.rs:965`
      (`every_part_two_spec_member_is_registered`, `:1137`, `:664`) and
      `crates/nvs-stdlib/tests/conformance_coverage.rs:155`
      (`every_core_class_has_a_conformance_floor_of_three`). `every_registry_row_carries_a_reference_card`
      is inline in `crates/nvs-stdlib/src/registry.rs`. These are standing gates, not new work: what
      can fail is a member landed without its three cases or its card.
- [ ] **The stage-5 check block, amended to what the tree holds** —
      `docs/agent/loop-goal.toml:5808` (the `exact` check over `examples/input-shapes.nvs`, confirmed
      green) and `:5822`. Same repair as stage 4's if a name here also describes a claim the corpus
      pinned under another spelling.
- [ ] **Whether the goal is done** — `docs/agent/loop-goal.toml:5837` is the last stage-5 check, and
      it passed by hand this session. With stages 2, 3 and 4 green the run may be one gate away from
      `DONE`; let the driver's own acceptance list decide rather than asserting it.

## Backlog

- Stage 4's `cargo-named` check no longer covers the taint diagnosis; the reject case does —
  `docs/agent/loop-goal.toml:5786`'s comment says why.
- `[context] modules` printed no `nvs-test` entry, so `crates/nvs-test/src/request.rs` — the `.nvsr`
  format the stage-5 fixture uses — was found by reading the example rather than from the pack.
