# Handoff

## State

**Goal 6, M7 — stage 9's `-p nvs-server` check (the load-bearing assertions) is whole.** All seven
names exist and pass; the two this session added are in `crates/nvs-server/src/serve.rs`'s test
module beside the five that were there:
`a_multipart_body_far_over_the_memory_bound_is_received_at_bounded_resident_memory` (`:3744`) and
`the_state_bleed_suite_passes_within_a_request_and_across_an_isolate_boundary` (`:4167`).

**The measured margin, so a future session does not re-derive it**: the body crossing holds
**90 KiB** at its peak while carrying 32 MiB — a 371st — and the case asserts against a 2 MiB bound,
which is headroom over `hyper`'s own 400 KiB read buffer rather than a measurement.

**Stage 9 is not finished**, and its three remaining checks are each a different kind of work:
`nvs-server (the memory floor)` names two tests that exist nowhere in the tree; the
`tools/bench.py --serve-vs-fpm --record benches/serve.json` command check has no `benches/serve.json`
to show; and the `differential` suite's `min_passing = 275` stands against 256 on disk.

## Next group

**The memory floor — ADR 0116 § 2's teardown sweep asserted from the door.** Both slices are one
`#[test]` each in `crates/nvs-server/src/serve.rs`'s test module, over the two fixtures this session
left there, and both read the same sweep in `crates/nvs-runtime`. Take them in this order: the first
is the single-request claim the second repeats under load.

- [ ] **A request that builds cycles returns its bytes at teardown** —
      `a_request_that_builds_cycles_returns_its_bytes_at_teardown`, ADR 0116 § 2: what the root drain
      leaves is swept when the arena is dropped, so a request holding a reference cycle still gives
      every byte back. The fixture shape is `crates/nvs-server/src/serve.rs:3634`'s `weigh_the_body`
      — a program that reports `nvs_runtime::budget::live_bytes()` and the door's own reading around
      it — and the sweep it is asserting is named at `crates/nvs-runtime/src/lib.rs:235`, with
      `crates/nvs-runtime/src/object.rs:1389` for what a cycle is. A program with no compiler in
      front of it builds one through `nvs_runtime::object`, not through source.
- [ ] **Live bytes are flat across a cycle-building soak** —
      `live_bytes_are_flat_across_a_cycle_building_soak`, the same claim under repetition: the
      reading after N requests is the reading after one, which is ADR 0004's "O(in-flight) rather
      than O(requests served)" on the request path. **N requests down one connection is the fixture**
      — `crates/nvs-server/src/serve.rs:4033`'s `across_a_request_boundary` is that shape already,
      with a `Cell` counting which run it is — and the playbook's bullet says a second *connection*
      is what a test cannot ask for.

## Backlog

- `benches/serve.json` does not exist; stage 9's `tools/bench.py --serve-vs-fpm --record` check
  wants `requests/sec` and `recorded` in its output. Its own file set is `tools/bench.py`.
- The `differential` suite is 256 against stage 9's `min_passing = 275`; `python tools/gaps.py`
  ranks the PHP twins with no oracle case.
- `hyper` disambiguates a request carrying both `Content-Length` and `Transfer-Encoding: chunked`
  where [ADR 0095](../adr/0095-ambiguous-input-is-refused-never-repaired.md) refuses ambiguous
  input. It is not a hole — the connection does not survive the message — but it is recorded only in
  a test comment in `crates/nvs-server/src/serve.rs`'s smuggling suite. Whether ADR 0095 or ADR 0097
  § 1 says so out loud is an ADR edit no session has made.
- `[context] adrs` in `docs/agent/loop-goal.toml` has no ADR 0116 section, and the next group is
  entirely about §§ 1-2 of it. Add `0116 §2` before that session opens the ADR by hand.
