# Handoff

## State

Goal `m8-stdlib-depth`. **Stage 13 is most of the way in.** `Core\Process::spawn` is registered and
built: it answers `Core\Process\Handle` with `readStdout`, `readStderr`, `writeStdin`, `wait` and
`kill`, every read and write goes off the core through `nvs_host::blocking::run`, and a child is
killed and reaped when its task's `Ctx` drops (`nvs_runtime::HeldChild`). Four of the stage's five
Rust tests exist and pass; three `.nvst` cases are on disk, including the acceptance's
`process-spawn-streams-a-childs-output-into-the-program.nvst`.

**Two acceptance items are open:** `a_spawn_produces_exactly_one_span` (no span is emitted anywhere
in `Core\Process` yet) and the two remaining acceptance artefacts — the concurrent-spawn perf guard
and the tainted-path/argv reject case. `rule:core-classes/process-spawn` is flipped to `shipped`.

Striking `Core\Process::spawn` emptied the last of the four spec-parity ratchets, so the whole
parity program is now at zero; the files stay for the next walk and
`every_outstanding_key_names_an_owner` no longer treats that as vacuity.

## Next group

**Stage 13, the tail: the span, the guard and the reject case** — one file set:
`crates/nvs-stdlib/src/process.rs` (the `spawn` helper at `crates/nvs-stdlib/src/process.rs:789`,
the tests module at `crates/nvs-stdlib/src/process.rs:1140`), plus
`benches/abi-probe/tests/perf_guards.rs:436` and one new file under `tests/conformance/reject/`.

- [ ] **A `spawn` produces exactly one span** — `docs/decisions/0076.md:372-373` puts a `spawn`
      beside a `query` and an outbound call; find what a `query` emits and emit the same thing once
      from `nvs_core_process_spawn` at `crates/nvs-stdlib/src/process.rs:789`, counted off the
      trace buffer in a Rust test named `a_spawn_produces_exactly_one_span` beside the four that
      landed at `crates/nvs-stdlib/src/process.rs:1140`. `rule:core-classes/process-spawn` is the
      member; the span model is whichever rule `python tools/brief.py --where span` routes to, and
      the goal's `[context] rules` does not carry it.
- [ ] **The concurrent-spawn scheduler guard** — `benches/abi-probe/tests/perf_guards.rs:436`
      measures one spawn and never several at once; add
      `concurrent_spawns_leave_the_scheduler_serving_other_tasks` beside it, on the shape
      `a_spawned_childs_reads_suspend_the_coroutine_and_free_the_core` at
      `crates/nvs-stdlib/src/process.rs:1257` uses — a neighbour task that must still run while
      several children are in flight. `rule:core-classes/process-spawn`.
- [ ] **A tainted `$path` or `$argv` element is refused where it is written** —
      `rule:core-classes/process-is-argv-only` says both are refused at the call site and only the
      shell-string half is pinned today
      (`tests/conformance/reject/there-is-no-shell-string-form-of-process-run.nvst`). One
      `--EXPECTF-ERROR--` case under `tests/conformance/reject/` named
      `process-run-refuses-a-tainted-path-and-a-tainted-argv-element.nvst`, asked of both members,
      since `spawn`'s row at `crates/nvs-stdlib/src/process.rs:130` carries the same `Qual::Sink`.

## Backlog

- `ProcessOptions` (cwd, env, timeout) lands on `run` and `spawn` together and is on neither —
  `rule:core-classes/process-options`, still `designed`.
- A second `wait` on a reaped handle answers the same status with empty captures, the first having
  taken the output — stated in `HANDLE_WAIT_DOC`, no case pins it.
- `crates/nvs-runtime/src/ctx/` is outside the goal's `[context] modules`, and this stage edited it
  (`held.rs`, `mod.rs`, `wiring.rs`); add `nvs-runtime/src/ctx/*` to the manifest.
- The parity ratchets are all at zero; whether the four files and their gates are retired is a
  decision nothing schedules — `crates/nvs-stdlib/tests/spec_registry_coverage.rs:326`.
- Stage 14's list of M8 verification items is unstarted — `docs/agent/loop-goal.md:235`.
