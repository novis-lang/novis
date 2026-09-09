# Handoff

## State

**Goal 23 — stage 5 is closed.** The `5 measured` check names two tests and both now exist and
pass: `ten_thousand_concurrent_cold_requests_for_one_file_compile_it_exactly_once`
(`crates/nvs-cli/src/script.rs:995`) and
`serve_throughput_scales_from_one_core_to_four_by_the_margin_this_test_names`
(`crates/nvs-cli/src/serve.rs:1539`). Stages 1–4 stay green.

The margin is named in the test and asserted against: **four cores serve at least 1.5× one core's
requests per second**, best of three interleaved rounds per side. The floor is where it is because
`nvs_host::cpus` enumerates *logical* CPUs — the four asked for are two physical cores on a machine
that pairs them — and the test's own doc comment is the home of that reasoning. It has teeth in
both directions, checked rather than argued: pinning all four workers to `cpus[0]` reports 1.32×
and fails, and sixteen busy threads saturating this box did not push a real fleet under the floor.

What the arms measure is every per-request cost above the socket — the fleet's one `Arc<Compiler>`
read and the isolate that unit runs as — and not the accept or the message parse, because a
loopback client fast enough not to be the bottleneck is a second fleet.

**A perf gap the number exposes and does not close:** on four *distinct physical* cores the ratio
is well under the ideal 4×, so something per request is shared. That is not this goal's item; the
backlog names it.

The rule still asks for one thing this tree does not do — that a compile runs on the worker that
needed it, not on a compile pool. That divergence predates this session and is unchanged.

## Next group

**Stage 5: what the number showed** — one file set: `crates/nvs-cli/src/serve.rs`, whose test
module now owns the measurement, with `crates/nvs-cli/src/script.rs` for the cache every request in
it reads.

- [ ] **Attribute the fleet's missing throughput** — the arm at
      `crates/nvs-cli/src/serve.rs:1448` scales well under `CORES` even when every worker has a
      physical core to itself, so a per-request cost is shared. The two candidates both sit on the
      request path: the `RwLock` read and `Arc` clone `Compiler::compiled`
      (`crates/nvs-cli/src/script.rs:386`) takes per request, and the allocation an isolate makes
      on the way up. Measure which, with the arm already written — an arm over a program body of
      `return 1;` isolates the fixed cost from the arithmetic — before proposing a change to
      either. `rule:http-server/the-accept-fan-out-is-one-worker-per-core` is what the number is
      about.
- [ ] **Say what a request costs where the crate says what it does** — whatever the arm above
      attributes belongs in `crates/nvs-cli/src/serve.rs:1` § *Known gaps* or `script.rs`'s module
      doc, not in a handoff, since it outlives this group.

## Backlog

- The compile still runs on a pool rather than on the worker that needed it —
  `rule:config/an-edit-reaches-the-next-request-without-a-restart`, `crates/nvs-cli/src/script.rs:1`.
- `PER_CORE` at `crates/nvs-cli/src/serve.rs:1409` is what makes an arm milliseconds; a faster
  runtime makes it too short and the constant, not the margin, is what moves.
- The measurement covers no accept and no message parse; an end-to-end throughput bench belongs to
  the benches, not to a `-p nvs-cli` test (`docs/agent/commands.md` § benches).
