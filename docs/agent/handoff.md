# Handoff

## State

**Stage 8's corpus half is closed.** All eleven `.nvst` cases the `conformance` check names in
`docs/agent/loop-goal.toml` are written and green; `tests/conformance/task/` is new and holds five of
them. Conformance is **986**, differential 200, and the check's other half — `min_passing = 1000` —
is the 14 cases still to come. `python tools/gaps.py` is the worklist for those.

**One code change came with the corpus**, and it is the smallest of the nine slices: `spawn script`'s
`args:` now reaches ADR 0033 § 4's refusal. `nvs_types::expr::quals::reject_secret_boundary_argument`
had said in its own doc that the spawn forms would come through it once they lowered; they lowered
and nothing connected them, so a `secret` value crossed to a child unrefused. The shared tail is
`reject_secret_crossing`, which both carriers call with a clause naming where the copy went — that
function's doc is the one home for why the message is shared rather than duplicated.

**Three gaps found while writing the cases**, none of them taken:

- An object does not cross the isolate boundary even when both files declare the identical class
  (playbook, *Writing a test case*). Same root as the plan's `Live::admit` gap.
- A `secret` value *inside an array* crosses both carriers unrefused — `quals::is_secret` reads the
  top-level qualifier only, and `nvs_runtime::graph`'s `field_is_secret` covers an object's
  properties, not an array's elements. `crates/nvs-types/src/expr/quals.rs` should own the note.
- `Core\Task::map`'s `fn` argument accepts a `callable` variable where `Core\Task::all`'s field is
  E0774. ADR 0072 § *Verification* names only the `all` side, so this may be intended; nothing says.

**Orientation gap, three sessions old and still unfixed:** `[context] modules` has no pattern for
`crates/nvs-cli/src/`. This session also needed `benches/abi-probe/`, which nothing in the manifest
selects — the previous handoff and the plan both said `benches/isolation.rs` did not exist and it has
existed all along, at `benches/abi-probe/benches/isolation.rs`.

## Next group

**Stage 8's benchmark half, and its guard — one file set: `benches/abi-probe/`.** The previous
handoff's framing was wrong about the tree; what follows is what is actually missing.

- [ ] **`isolation.rs` measures a proxy, not an isolate.** Its two arms are `os_process/noop` and
      `task/create_and_finish` (`benches/abi-probe/benches/isolation.rs:29`, the group; its module
      doc at line 17 says the real figure "belongs here next to the baseline it beats" once M5/M6
      land — they have). Add the third arm: `nvs_host::Isolate::new(...).run(ctx)`, the same three
      lines as `crates/nvs-cli/src/runner.rs:797`. `nvs-abi-probe` depends on neither `nvs-host` nor
      `nvs-runtime` today, so this slice starts in `benches/abi-probe/Cargo.toml:47` where the four
      `[[bench]]` sections are. M5's acceptance (`python tools/plan.py --show M5:verify`) is the
      specification.
- [ ] **`a_spawn_to_result_round_trip_stays_in_the_microsecond_class` does not exist.** It is the
      second of the three names in the `abi-probe (isolation cost)` check
      (`docs/agent/loop-goal.toml:1675`); the other two are green.
      `benches/abi-probe/tests/perf_guards.rs:423` is
      `an_os_process_costs_orders_of_magnitude_more_than_a_task`, the shape to copy, and
      `ns_per_op` at line 50 is the helper every guard there measures through.
- [ ] **The corpus count, after those two.** 986 of 1000, so 14 cases. `python tools/gaps.py` ranks
      the candidates; `docs/agent/conventions.md` § *A `.nvst` test case* names the four shapes a
      depth case takes.

## Backlog

- A `secret` inside an array crosses both graph-copy carriers unrefused —
  `crates/nvs-types/src/expr/quals.rs`.
- An object cannot cross the isolate boundary at all — `crates/nvs-runtime/src/graph.rs` § *Known
  gaps*.
- `Core\Task::map` accepts a `callable` variable where `::all` refuses one — ADR 0072 § *Verification*.
- Item 18's `Core\Secret::reveal()` is not in the registry, so the refusal above has no way out —
  `nvs_types::expr::quals`.
- Item 22's `Core\Script` members are unwritten, so a child cannot read its `args:` —
  `crates/nvs-stdlib/src/script.rs`.
- `[context] modules` selects neither `crates/nvs-cli/src/` nor `benches/abi-probe/` —
  `docs/agent/loop-goal.toml`.
