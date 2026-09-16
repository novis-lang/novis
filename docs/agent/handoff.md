# Handoff

## State

Goal `m8-stdlib-depth`. **Stage 14's three `-p nvs-stdlib` checks are closed.** All three tests are on
disk and green: the four-grammar agreement test in `crates/nvs-stdlib/src/lib.rs`, the per-core local
tier in `crates/nvs-stdlib/src/cache.rs`, and the protocol roster's no-accessor claim in
`crates/nvs-stdlib/src/registry.rs`.

Nothing in the runtime or the registry changed — each slice is a test written over code already on
disk, so the stage cost no design call and opened no ADR. Stage 14's two remaining checks are in
other crates, and its third prose item (the cache bullet) still has no check at all.

## Next group

**Stage 14: the remainder, outside `nvs-stdlib`** — two test files that already exist, one item each,
plus the prose item that is evidence rather than a test:

- [ ] **A statement holding a folded intrinsic call is still reported covered** — goal prose stage 14
      item 2, over `rule:expressions/preparation-preserves-behaviour`, whose last paragraph is the
      whole claim: a fully folded call stops probing, and the statement's own probe is unaffected.
      The test is `a_statement_holding_a_folded_intrinsic_call_is_still_reported_covered` and it goes
      beside `crates/nvs-codegen/tests/probes.rs:140`; the emission it is about is
      `crates/nvs-codegen/src/lib.rs:1542`.
- [ ] **Every literal pattern in the regex suite has its tier recorded** — goal prose stage 14 item 4,
      over `rule:core-classes/regex-literal-tiering`. The fold writes the tier at
      `crates/nvs-types/src/expr_table.rs:1735` and the suite to sweep is
      `crates/nvs-types/tests/intrinsics.rs:206`; the test is
      `every_literal_regex_pattern_in_the_suite_has_its_tier_recorded`.
- [ ] **The cache bullet is struck with evidence or becomes a test** — goal prose stage 14 item 3,
      which `docs/agent/loop-goal.toml:11124`'s stage header says carries no check. The evidence the
      backlog already names is `crates/nvs-cli/src/cache.rs:2505`.

## Backlog

- Stage 15, the rulebook — `docs/agent/loop-goal.toml:11155`.
- `ProcessOptions` is shipped on neither member — `rule:core-classes/process-options`.
- A spawn's event is left unjoined when the program never waits, which is also what a cancelled child
  reads as; nothing distinguishes the two — `rule:observability/spawn-is-its-own-event`.
- 0059's M8 list also asks that the same value via the *shared* tier is present; the local half is now
  asserted and the coherent counterpart is only the process tier's
  `process_tier_is_one_map_every_core_reads` — `docs/decisions/0059.md:163-164`.
