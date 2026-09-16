# Handoff

## State

Goal `m8-stdlib-depth`. **Stage 14's `-p nvs-codegen` check is closed** —
`a_statement_holding_a_folded_intrinsic_call_is_still_reported_covered` is at
`crates/nvs-codegen/tests/probes.rs:147` and green, over an attribute retrieval whose registered
body aborts on entry, so a run that returns is a run whose call folded.

Two carried `1 floor` checks were red and are green again. Neither was a regression: the goal file
had gone stale under landed decisions. `examples/uncaught.nvs`'s `want` now reads the JSON frame
objects `rule:errors/record-producers` fixes, and `examples/reflect.nvs` counts
`readableProperties()` now that `properties()` is the complete roster. The metrics module path in
`docs/agent/goals/59-m8-stdlib-depth.toml` is back in sync with the live copy, so the two are
byte-identical again.

Stage 14's remainder is items 3 and 4 of the goal prose; nothing is blocked.

## Next group

**Stage 14: the remainder** — one file set: `crates/nvs-types/tests/intrinsics.rs` and the committed
tier record it writes, with `crates/nvs-cli/src/cache.rs` for the evidence item beside it.

- [ ] **Every literal pattern in the regex suite has its tier recorded** — goal prose stage 14 item 4,
      over `rule:core-classes/regex-literal-tiering`, whose second paragraph is the claim: the tier is
      a property of the pattern text alone, so writing it to a committed file makes an engine change
      that moves a pattern show up in a diff. The test is
      `every_literal_regex_pattern_in_the_suite_has_its_tier_recorded` and it goes beside
      `crates/nvs-types/tests/intrinsics.rs:206`, which settles one pattern's tier and records nothing;
      the reasoning is `docs/decisions/0056.md:144-145`.
- [ ] **The cache bullet is struck with evidence or becomes a test** — goal prose stage 14 item 3, over
      `rule:expressions/preparation-preserves-behaviour`'s last paragraph, which keys a prepared
      artifact on the compiler-environment component. Read `crates/nvs-cli/src/cache.rs:2505`: if a
      prepared entry rides inside the payload that test already keys, the item is struck in
      `docs/agent/loop-goal.md` with that citation and owes no test. It has no `[[check]]` of its own
      either way.

## Backlog

- Stage 15 flips `core-classes/process-spawn`, `observability/metrics-three-members` and one more rule
  to shipped — `docs/agent/loop-goal.md` § Stage 15.
- `[context] modules` is 21 entries against an 18 ceiling; narrowing it is left for a person —
  `docs/agent/loop-goal.toml`.
