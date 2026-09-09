# Handoff

## State

**Goal 23 — stage 3's fan-out is on disk and all five of its `nvs-cli` checks are green. What is
left of stage 3 is the drain, which is `-p nvs-server` and a different file set.** Stages 1 and 2
stay green.

`nvs serve` binds every `[server] listen` entry before any worker exists and then runs one worker
per core, each holding its own handle on every listener —
`rule:http-server/the-accept-fan-out-is-one-worker-per-core`, whose status this session flipped from
`designed` to `shipped` on ADR 0161's own instruction.

**What landed in `crates/nvs-cli/src/serve.rs`:** `addresses` answers the whole configured set
rather than its first entry and takes the Unix-domain refusal once over it; `bind_all` binds `std`
listeners at the boot; `handles_for` duplicates each of them once per worker; `serve_on_worker` is
one core's whole server — its own `Table`, its handler, one accept task per listener, its reactor
and its scheduler — and `run` spawns the workers over `nvs_host::cpus()` and joins them for the exit
code. A host that enumerates no CPU has no `CpuId` to pin to and is served from the boot thread.

**Two things worth knowing before the next change here.** The `[[schedule]]` ticker rides worker 0
and arms *there* rather than at the boot, because `nvs_server::Armed` holds an `Rc` (playbook, §
*Running things*). And nothing asserts that a connection is taken by whichever core reaches it
first: the tests assert the handles and a hand-run `nvs serve` answered six requests across the
fan-out, so stage 5's throughput check is still the measurement.

## Next group

**Stage 3: the drain is fleet-wide** — one file set: `crates/nvs-server/src/serve.rs`, with
`crates/nvs-runtime/src/drain.rs` read for the process's own bit. Nothing in `nvs-cli` is in this
group.

- [ ] **The drain's two ends, `is_draining_answers_the_same_on_every_core` and
      `the_process_exits_when_the_last_cores_in_flight_count_reaches_zero`** — `Draining` is the
      process's bit and every worker takes its own handle on it
      (`crates/nvs-server/src/serve.rs:318`, over `crates/nvs-runtime/src/drain.rs:58`), so the
      first half is that two handles answer alike; the second is the in-flight count each loop
      keeps (`crates/nvs-server/src/serve.rs:1288`) reaching zero on the last core rather than on
      one. `rule:http-server/the-accept-fan-out-is-one-worker-per-core` § *what a core holds of its
      own* is the specification.
- [ ] **The backoff, and `the_accept_backoff_runs_per_core_and_one_cores_backoff_does_not_stall_another`**
      — `crates/nvs-server/src/serve.rs:1458` (`AcceptBackoff`) is per accept loop already; the
      case is that one loop's wait does not hold another's, which is
      `rule:http-server/the-accept-loop-backs-off` read across cores.
- [ ] **`a_disconnected_clients_isolates_are_left_behind_on_no_core`** — a peer that goes away must
      leave nothing behind on any core; the connection's own task is the root the isolates hang off
      (`crates/nvs-server/src/serve.rs:1288`, the outstanding counter, and the accept loop above
      it).

## Backlog

- Stage 4's state-bleed suite across a core boundary, and the fleet-wide in-flight ceiling —
  `docs/agent/loop-goal.toml`, stage 4.
- Stage 5's measurement: `serve_throughput_scales_from_one_core_to_four_by_the_margin_this_test_names`.
- `--port` over a set mixing TCP and Unix-domain entries collapses to one loopback address;
  `addresses` dedupes it rather than binding twice — `crates/nvs-cli/src/serve.rs:@addresses`.
- The per-core watchdog `rule:http-server/a-wedged-core-is-detected-by-its-deadline` names is still
  written against one core.
