# Handoff

## State

**Stage 7 is closed: all four names in the goal's `7 test isolates` check pass.** Item 26 — ADR
0079 § 12's fixed clock and seed — landed on top of item 25's task tree. Both are the same shape:
a cold field on `nvs_runtime::Ctx` (`fixed_clock`, `random_state`), written by
`crates/nvs-cli/src/runner.rs` onto the *child's* context inside `run_in_isolate`'s program
closure and by nothing else. Those two field doc comments are the one home for why this does not
reopen ADR 0008, and for why a seedable generator does not weaken `Core\Random`: no spelling
outside a `#[Test]` reaches either field.

**Every reading goes through one seam per domain**, which is what makes the two options
all-or-nothing rather than per-member: `nvs_stdlib::time::wall_clock` (read by `Core\Time::now`
and `Core\Uuid::v7`'s timestamp half) and `nvs_stdlib::random::draw` (all seven `Core\Random`
members and both `Core\Uuid` ones). `Core\Time::monotonic` is deliberately *not* fixed —
`nvs_core_time_now`'s doc comment owns that call.

**`Core\Test::advance(Duration)` is the new `Core` member**, five edits in
`crates/nvs-stdlib/src/test.rs`, with both refusals on one `Core\Test::advance(): {why}` prefix.
Guards: `a_test_at_a_fixed_clock_reads_that_clock` and
`a_test_with_a_seed_draws_the_same_sequence_twice` over
`crates/nvs-cli/tests/fixtures/runner/{fixed-clock,seeded}.nvs`, plus three conformance cases
under `tests/conformance/core/` for the refusal side. `seeded.nvs` freezes seed 42's answer, so
changing SplitMix64's step is meant to fail there. Conformance is 977.

**Orientation gap, unchanged and now two sessions old:** `[context] modules` has no pattern for
`crates/nvs-cli/src/`, so the map block prints nothing for `runner.rs` — the file every Stage 7
slice has edited. Add that selector.

## Next group

**Stage 8 is what is left of this goal, and its two halves share no files.** Take the benchmark
and its guard together — same subject, same acceptance paragraph, `docs/plan/m5.md` § *Verify* is
the specification for both.

- [ ] **`benches/isolation.rs` does not exist.** M5's acceptance asks for spawn-to-result for a
      trivial child on a warm cache, committed next to the process baseline it replaces.
      `benches/abi-probe` is the shape to copy, and the isolate under measurement is the
      `nvs_host::Isolate::new(...).run(ctx)` at `crates/nvs-cli/src/runner.rs:797` — the same
      three lines the test runner already uses.
- [ ] **The guard beside it: `an_os_process_costs_orders_of_magnitude_more_than_a_task`.** Named
      in `docs/plan/m5.md` § *Verify* and in no test file yet. A test, not a bench, so it asserts
      an order of magnitude rather than a number.
- [ ] **The corpus count, as a session of its own.** The `conformance` check wants
      `min_passing = 1000` and the tree is at 977. `python tools/gaps.py` ranks the candidates and
      conventions.md holds the four depth-case shapes. It shares no file with the two above.

## Backlog

- Item 18's `Core\Secret::reveal()` is not in the registry, so the boundary's `secret` refusal has
  no way out — `nvs_types::expr::quals`.
- `Live::admit`'s same-class check is asked of the answer and not of the argument —
  `crates/nvs-runtime/src/graph.rs` § *Known gaps*.
- Item 22's `Core\Script` members are unwritten — `crates/nvs-stdlib/src/script.rs`.
- `Core\Random\Seeded` is still unbuilt — `crates/nvs-stdlib/src/random.rs` § *Known gaps*. § 12's
  test seed does not substitute for it: that is a production type, this is isolate config.
- `[context] modules` needs a `crates/nvs-cli/src/` pattern — `docs/agent/loop-goal.toml`.
