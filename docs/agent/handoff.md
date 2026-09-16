# Handoff

## State

Goal `m8-stdlib-depth`. **Stage 14's `-p nvs-types` check is closed** —
`every_literal_regex_pattern_in_the_suite_has_its_tier_recorded` is at
`crates/nvs-types/tests/intrinsics.rs:471` and green, over the committed record
`tests/conformance/core/regex-literal-tiers.txt`: 45 patterns, three of them backtracking and two
refused by both engines on purpose.

The record covers the whole suite rather than the folded part of it. The intrinsic roster carries
`Core\Regex::compile` alone, so the test finds each pattern argument through `nvs_syntax::walk` and
`nvs_stdlib::regex::CLASS`'s rows, then settles its tier by handing the literal to `compile` — one
implementation throughout, per `rule:expressions/preparation-preserves-behaviour`. A per-case count of
the calls the walk reached against the calls the case's text shows is what keeps a missed call loud.
`ExprTypeTable::regex_tier_sites` is new and is what the fold's own settlements are cross-checked
through; `check_src_table_allowing_errors` is its `tests/common/mod.rs` sibling, for source the caller
did not write.

Stage 14's remainder is item 3 of the goal prose alone. Nothing is blocked.

## Next group

**Stage 14: the last item** — one file set: `crates/nvs-cli/src/cache.rs` and the goal file's own
stage-14 block.

- [ ] **The cache bullet is struck with evidence or becomes a test** — goal prose stage 14 item 3, over
      `rule:expressions/preparation-preserves-behaviour`'s third paragraph, which keys a prepared
      artifact on the compiler-environment component so a different build is a miss rather than a
      mismatch. `crates/nvs-cli/src/cache.rs:2505` proves it for the whole payload; the question is
      whether prepared entries ride inside that payload, and if they do the bullet is struck with that
      evidence and owes no test. The reasoning is `docs/decisions/0057.md`'s cache bullet.
- [ ] **Stage 14's two sibling checks re-run green** — `docs/agent/loop-goal.toml:11126` and `:11142`
      name them (`-p nvs-stdlib`'s three tests, `-p nvs-codegen`'s one). They landed in earlier
      sessions and the driver has not reported them since; confirm rather than rebuild.

## Backlog

- A malformed literal pattern at `matches`/`split`/`replace` is a run-time throw, not a compile error,
  which `rule:core-classes/regex-literal-tiering`'s first paragraph reads as covering — the division is
  pinned by `tests/conformance/core/regex-compile-answers-a-pattern-every-member-reads-as-its-source-string.nvst:98`
  and is a roster change with its own fixture, not this goal's.
- `crates/nvs-types/src/derive.rs:72-83` gap 2 and `crates/nvs-cli/src/worker.rs:101` — goal `m8-db-queue`.
- `crates/nvs-stdlib/src/ast.rs:52` gap 2 and `crates/nvs-stdlib/src/json.rs` gaps 2–5 — goal `unowned-closures`.
- The M6-tagged `crates/nvs-stdlib/src/regex.rs:77` and `:91` gaps — goal `unowned-closures`.
