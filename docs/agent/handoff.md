# Handoff

## State

**Goal 23 — stage 5 is half closed.** The `5 measured` check names two tests;
`ten_thousand_concurrent_cold_requests_for_one_file_compile_it_exactly_once` now exists and passes,
and `serve_throughput_scales_from_one_core_to_four_by_the_margin_this_test_names` does not exist
yet. Stages 1–4 stay green.

The module doc's first known gap is closed: `crates/nvs-cli/src/script.rs` single-flights a compile.
`CompileState` has the third state `rule:config/an-edit-reaches-the-next-request-without-a-restart`
specifies, one caller claims a content's key under the write guard and compiles it, and every other
caller waits on the `Flight` it left in the table and is then answered out of that table. The wait
is a condition variable, so it stops a worker thread for as long as one compile — which is what it
replaces, that same worker running the same front end itself. `Landing` is a drop guard, so a panic
beneath the front end wakes the waiters instead of wedging them; what they find then is a table with
nothing under the key, which sends them to compile it themselves.

Measured, not argued: with `Claim::Behind` short-circuited to `Claim::Mine` the new test reports 4
compiles, and with it in place, 1.

The rule still asks for one thing this tree does not do — that compile runs on the worker that
needed it, not on a compile pool. That divergence predates this session and is unchanged.

## Next group

**Stage 5: the number** — one file set: `crates/nvs-cli/src/serve.rs`, whose test module already runs
a booted fleet, with `crates/nvs-cli/src/script.rs` for the cache the workers share.

- [ ] **`serve_throughput_scales_from_one_core_to_four_by_the_margin_this_test_names`** — the second
      and last test the `5 measured` check names, and nothing in the tree measures throughput yet:
      `crates/nvs-cli/src/serve.rs:1103` is where its test module opens,
      `crates/nvs-cli/src/serve.rs:383` is the fan-out frame that builds one watchdog and one
      `Arc<Compiler>` (`crates/nvs-cli/src/serve.rs:307`) for N cores,
      `crates/nvs-cli/src/serve.rs:475` is `serve_on_worker` — the whole of what one core does — and
      `crates/nvs-cli/src/serve.rs:436` is the `Core` a worker is handed. One core is the
      `serve_on_worker` call at `crates/nvs-cli/src/serve.rs:372`, which runs on the caller's own
      scheduler; four is `nvs_host::Worker::spawn` at `crates/nvs-cli/src/serve.rs:402`.
- [ ] **Name the margin in the test, and make it survive a loaded box** —
      `crates/nvs-cli/src/serve.rs:402` is what the four-core half is measured over. The check's own
      name says the test names its margin, so the number is that slice's decision and belongs in the
      test's comment beside what it was measured against. The loop driver is compiling on this
      machine while the test runs, so a margin near 4× will flake; one that only says "more than a
      single core's worth" is the safe option, and `AGENTS.md` § *Session workflow* wants the safe
      option taken and said rather than a `BLOCKED`.

## Backlog

- A compile still runs on the worker that needed it —
  `rule:config/an-edit-reaches-the-next-request-without-a-restart` says "on the compile pool, never
  on a request-serving core". No pool exists anywhere in the tree; unowned, and not stage 5's.
- `docs/plan/m7.md`'s acceptance paragraph is what both stage-5 tests are filed against; nothing
  reads it into the plan's status block.
