# Handoff

## State

**Goal `m5-proofs` (M5). Stage 7 `the speedup` is complete: both of its acceptance checks are
green**, and M5's near-linear claim is now measured rather than asserted.

`benches/abi-probe/shared/isolate.rs` carries the fan-out beside `spawn_to_result_batch` —
`worker_fan_out_batch` places `WIDTH` (four) CPU-bound children through `nvs_host::worker::post`
and joins them, `one_core_batch` runs the same four where they stand, and one driver builds the
scheduler, the reactor and the task outside the clock for both. The claim is the ratio between
them, so both halves are one measurement and the bench has both rows. A refused placement panics
rather than quietly running the children here, which is the one way this measurement could report
a speedup it did not have.

Measured release, x86_64-pc-windows-msvc: 0.9 ms placed against 3.4 ms on one core, **3.92x** of an
ideal 4x. The guard's floor is 2.0x — `benches/abi-probe/tests/perf_guards.rs` says why the margin
is half the ideal rather than near it.

**Stage 8 `tsan` is next and none of it is written**: there is no `tools/tsan.sh`,
`.github/workflows/ci.yml:460` still says TSAN is owed, and no stack switch carries a fiber
annotation. All four of its checks are the first kind of failure — an artefact that does not exist
yet.

## Next group

**Stage 8: tsan** — one file set: `tools/tsan.sh`, `.github/workflows/ci.yml`,
`crates/nvs-host/src/scheduler.rs`.

- [ ] **`corosensei`'s stack switches carry TSAN's fiber annotations** —
      `crates/nvs-host/src/scheduler.rs:894` is the resume, and the yield is the `Yielder` beside
      it; the sanitizer sees one thread's stack become another and reports every task boundary
      without them. `#[cfg(sanitize = "thread")]` so it is compiled out of every build but the
      leg's, which is the goal's § *Standing decisions* naming this as the only new `unsafe` here.
- [ ] **`tools/tsan.sh` is the job's own command and ends on `tsan: clean`** — the contract is
      `docs/agent/loop-goal.toml:9942`, which runs it under WSL because the driver is on Windows
      (`docs/agent/commands.md` § *WSL*). Nightly, `-Zsanitizer=thread` over `nvs-host` and
      `nvs-runtime`, and the fallback if an annotated switch still reports is in the same standing
      decision: narrow to the tests that cross threads without switching stacks, and say in the
      script what it skips.
- [ ] **The workflow gains a `tsan:` job and drops the note that owes it** — the three checks that
      read it are `docs/agent/loop-goal.toml:9918`, and what they read is
      `.github/workflows/ci.yml` line 460, the comment saying TSAN is owed, with the `miri:` job
      under it as the shape to follow. CI is not running, so the job is proven by reading the file;
      the sanitizer itself is proven by the check above.

## Backlog

- `nvs serve` places a child on a lazily started worker core rather than a sibling serving core —
  `crates/nvs-host/src/worker.rs`'s `# Known gaps`, the fallback ADR 0184 § 5 pre-authorizes.
- A `spawn script` *path* entry still runs on the parent's core; only a method crosses —
  `crates/nvs-host/src/placed.rs`'s `# Known gaps`.
- No end-to-end `nvs serve` runaway test for the CPU and memory ceilings —
  [carried-gaps.md](carried-gaps.md).
- `docs/plan/m5.md`'s Verify sentence still puts the speedup on `Core\Task::map`; goal
  `plan-truth` owns that wording.
