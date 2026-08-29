# Handoff

## State

**Every `Core` registry entry now carries its ADR 0117 reference card** — 347 members, 11 enums, every
constant — and `every_registry_row_carries_a_reference_card` in `crates/nvs-stdlib/src/registry.rs`
refuses a new one without. The rule a future member follows is
[conventions.md](conventions.md) § *A `Core` member — the five edits*, edit 2; ADR 0117 § 1 records
that the fields stopped being optional. Every card's `errors` was read from the helper body, not the
prose, so the cards are honest about the classes the code throws today — mostly `RuntimeError`, since a
bare `Fault::thrown` is that class — and the disagreements that reading surfaced are in the backlog
below, unresolved. The website's reference has already taken them: `npm run sync:core` ran after the
backfill, its scan repaired to read a row that carries `names`, and every draft page and
`website/src/data/core.json` are regenerated from the cards.

**Stage 0b's residue is closed: ADR 0063 R2 holds end to end, including for a parameter whose spec
name is a reserved word.** `crates/nvs-syntax/src/parser/expr.rs`'s `parse_arg` reads any word before a
`:` as an argument name, keyword or not, so `Core\Arr::map(fn: …, a: …)` parses and the seven
`fn`-named rows need no rename. The rule itself lives in `parse_arg`'s doc comment.

**Stage 7 is the open front, and its acceptance check is aimed correctly.** The `7 test isolates` check
in `docs/agent/loop-goal.toml` runs `-p nvs-cli`, and `docs/agent/goals/2-concurrency.toml` is
byte-identical to it. All four of its names are still unwritten — that is item 25 itself, and it is the
next group below.

**Orientation gap, unchanged:** `[context] modules` has no pattern for `crates/nvs-cli/src/runner.rs`,
which the whole next group is written against, nor for `crates/nvs-syntax/src/parser/expr.rs`. Add both
selectors before the next session opens this group.

## Next group

**All three slices share `crates/nvs-cli/src/runner.rs` and `crates/nvs-host/src/isolate.rs`; they
are Stage 7 item 25 and ADR 0079 §§ 2 and 16, and together they turn the goal's `7 test isolates`
check green.**

- [ ] **One isolate per `#[Test]`.** `crates/nvs-cli/src/runner.rs:190` builds one `Ctx` for the
      whole run and `:436` is the per-test entry; `:48` and `:299` are the two comments still saying
      isolate-per-test is M5's and unbuilt. `crates/nvs-host/src/isolate.rs:90` is `Isolate`, whose
      program arrives as a closure rather than a path, so the runner — which already holds the
      compiled unit — is the one place that closure can be built. The guard is
      `each_test_runs_in_its_own_isolate_sharing_only_compiled_code`, and what it must observe is a
      static written by one test that the next one does not read back (ADR 0116 § 4's fresh statics
      base).
- [ ] **The runner owns the test's task tree.** Same file: a test's isolate is a child task, so a
      `Core\Task::all` inside a `#[Test]` has a calling task to be a child of, and nothing the test
      started is still running when its row is reported (ADR 0072 § 4). Guard:
      `the_runner_owns_the_tests_task_tree`.
- [ ] **A fixed clock and a seed are per-test.** Guards `a_test_at_a_fixed_clock_reads_that_clock`
      and `a_test_with_a_seed_draws_the_same_sequence_twice`; both are properties of the child
      context the slice above builds, so they are cheap once it lands and expensive before it.

## Backlog

- Item 22's `Core\Script::valueOrThrow` is still unwritten — `docs/plan/m5.md`, Stage 6.
- `Core\Secret::reveal()` is absent from the registry, so ADR 0033's escape hatch is open at both
  ends — `crates/nvs-stdlib/src/registry.rs`.
- The graph copy asks its class-identity question of the answer but not of the argument; closing it
  means the `nvs_runtime::script` seam answering with a unit's class table
  (`crates/nvs-runtime/src/graph.rs` module doc).
- M4's 1000-case conformance floor — `docs/implementation-plan.md`, Stage 8.
- `python tools/check-migration.py` reads 34% classified against a 100% floor — Stage 8.
- **Spec versus code, found by the card backfill; each needs one of the two changed.** Spec § 2 says
  `by` and `comparator` on `Core\Arr::sort`/`diff`/`intersect` are mutually exclusive and a compile
  error; `nvs_core_arr_sort`'s doc says they compose and the code composes them.
  `website/src/content/docs/docs/core/str/format.mdx` promises `ParseError` for a malformed run-time
  template; `crates/nvs-stdlib/src/format.rs` throws `RuntimeError`. `$dt->format`, `$d->format` and
  `$t->format` throw `RuntimeError` for a bad pattern where `Core\Time::parse` throws `LogicError` for
  the same failure. Every `Core\Math` throw is `RuntimeError` where spec § 3 and ADR 0007 § 4 say
  `ArithmeticError` — `math.rs`'s module doc already records that one. The cards follow the code in
  all four.
- `npm run sync:core` warns six times that the registry documents a parameter the spec does not
  declare — `$b` on `Core\Encoding::fromBase64`/`fromBase64Url`/`fromHex`/`fromBase32`, `$d` on
  `Core\Time\Duration::plus`/`minus`. The spec's combined rows (`toBase64` / `fromBase64`, one
  signature) carry no signature for the second member, so the website's parser sees no name where
  the Rust guard already skips the row. Either the spec writes both signatures or
  `website/scripts/lib/spec.mjs` learns the combined row.
