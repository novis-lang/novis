---
milestone: M5
---
# Loop goal 55 — the scheduler's claims are proven at the scale M5 promised them

Every claim M5's *Verify* paragraph makes about the scheduler and the isolate is a test that runs: a
hundred thousand tasks in flight on one core, a deliberate deadlock ended by the cancellation it
exists to prove, a CPU-bound fan-out that gets faster with every core it is given, and ThreadSanitizer
clean over the crates that cross threads. The two halves of M5's scope that were parsed but never
built — `spawn script`'s `on:`, `limits:` and `grants:` options, and the `spawn` trace event — are
built, and a child isolate that asks for request state is told there is none.

## Why here

After goal `m4-refusals`, because the chain pays the milestones' owed work in milestone order. Before goal
`m4b-editor` and the M7/M8 goals, because every later surface that runs a child — a job, a
connection, a `Core\Socket::upgrade` with its own `on:` (goal `server`'s item 19b) — is written
against the isolate this goal finishes, and a placement option added after them is an option each of
them grows separately. Before goal `gap-zero` for that goal's standing reason: a register is emptied
after everything that adds to it has run.

What it needs already built: the scheduler and its task tree (`crates/nvs-host/src/scheduler.rs`), the
task group and its deadline (`crates/nvs-host/src/group.rs:137`), `Core\Task\Channel`
(`crates/nvs-host/src/channel.rs`), the isolate and its two crossings
(`crates/nvs-host/src/isolate.rs:105`, `:401`), the cross-thread wake
(`crates/nvs-host/src/reactor.rs:219` `RemoteWake`), the blocking pool whose handoff shape a worker
placement copies (`crates/nvs-host/src/blocking.rs:12-20`), the `TRACE`/`PROFILE` bits
(`crates/nvs-runtime/src/ctx/mod.rs:254-257`), and the spawn-to-result guard beside the process baseline
(`benches/abi-probe/tests/perf_guards.rs:486`).

## Stage 0 — the catch-up

Sentences on disk this goal makes wrong. Each is corrected in the stage that makes it wrong. Re-grep
before editing: these are anchors, and files move.

- `.github/workflows/ci.yml:460-463` — "TSAN is still owed, and arrives with M5's concurrency." Stage 8
  adds the leg and rewrites that banner whole.
- `crates/nvs-runtime/src/ctx/trace.rs:59-64` — `Spawn` is "unrecorded for the same reason as
  `TraceKind::Gc`". Stage 4 records it; the `Gc` half of the sentence stays true and is rewritten
  standing alone.
- `crates/nvs-types/src/expr/isolate.rs:143-160` — `E_SPAWN_OPTION_UNSUPPORTED` refuses `limits:`,
  `grants:` and `on:` as "not enforced yet". Stage 6 enforces all three and retires the diagnostic,
  doing with its code whatever `crates/nvs-diagnostics` already does with a retired one.
- `crates/nvs-host/src/stack.rs:15-16` — "100k concurrent tasks is 100 GiB of *address space*" is
  true and incomplete on Linux: stage 2 adds the map-count fact beside it.

## Stage 1 — the floor

Goal `m4-refusals`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the scale and the deadlock

One file set: `crates/nvs-host/src/scheduler.rs`, `crates/nvs-host/src/group.rs`,
`crates/nvs-host/src/channel.rs`, `crates/nvs-host/src/stack.rs`, and one `.nvst` case under
`tests/conformance/task/`.

- **A hundred thousand tasks in flight on one core** — beside
  `many_tasks_can_be_created_and_driven` (`crates/nvs-host/src/scheduler.rs:1934`), which is a
  thousand. Every task is created before the first finishes, each suspends twice, and the run asserts
  the finished count, the resume count and that the pool kept `MAX_POOLED_STACKS`
  (`crates/nvs-host/src/stack.rs:71`) and no more. It is **release-only**, marked the way
  `serve_throughput_scales_from_one_core_to_four_by_the_margin_this_test_names` is
  (`crates/nvs-cli/src/serve.rs:1812-1817`): a hundred thousand stacks touched in the debug profile, beside
  every other test binary `nv verify` runs, is a resident cost that measures the machine. The driver's
  release slot runs it. `rule:concurrency/a-task-stack-is-reserved-wide-and-pooled`.
- **The Linux map count.** `corosensei` 0.2.2 reserves a stack as one `PROT_NONE` mapping and then
  `mprotect`s the usable part, so every stack in flight is **two** kernel mappings, and Linux's default
  `vm.max_map_count` of 65530 caps a process at about 32k tasks in flight. On Linux the test reads
  `/proc/sys/vm/max_map_count` first and fails naming the sysctl and the value it needs, rather than
  dying inside `mmap`. The fact goes in `crates/nvs-host/src/stack.rs`'s module doc beside the 100 GiB
  sentence, and the line an operator runs goes in `docs/setup.md`. A stack the OS refuses stays a
  panic: `crates/nvs-host/src/scheduler.rs:755-757` decided that, and this goal does not re-open it.
- **A deliberate deadlock, ended by the cancellation it proves.** Two children of one `Task::all`,
  each `receive`-ing on a `Core\Task\Channel` only the other would `send` to, under a `deadline`: the
  deadline cancels both, the call waits for both teardowns, throws `TimeoutError`, and leaves no child
  running and no waiter registered on either channel. Twice — once in Rust against `group.rs`'s
  deadline (`crates/nvs-host/src/group.rs:771`'s neighbour), once as a `.nvst` case a program can read —
  and a third time with no deadline, ended by cancelling the task that awaits the pair.
  `rule:concurrency/nothing-is-still-running-when-a-call-returns`'s table is what each asserts.

## Stage 3 — the isolate proofs the Verify paragraph names

One file set: `crates/nvs-host/src/isolate.rs`, `crates/nvs-host/tests/limits.rs`,
`crates/nvs-stdlib/src/session.rs`, and `.nvst` cases under `tests/conformance/isolate/` and
`tests/conformance/core/`.

- **Request state throws in a child** — `rule:security/request-state-throws-in-an-isolate`. A
  `spawn script` child never has a request: `crates/nvs-host/src/group.rs:197` builds it with
  `Isolate::new` and never calls `Isolate::answering` (`crates/nvs-host/src/isolate.rs:239`), so
  `Core\Request` already throws there through `crates/nvs-stdlib/src/request.rs:1690`. Nothing proves
  it. One `.nvst` case spawns a child that reads `Core\Request::method()` and starts a session, and
  prints both refusals.
- **`Core\Session::start` does not throw where no request arrived** — it reads the cookie only
  `if` there is an inbound request (`crates/nvs-stdlib/src/session.rs:771`) and otherwise opens a
  fresh session. It is made to throw the `LogicError`
  `crates/nvs-stdlib/src/request.rs:1712`'s `no_request` builds, **after** the configuration questions
  `start` already asks first (`crates/nvs-stdlib/src/session.rs:743-760`), so the frozen floor cases
  that call `start` with no store configured keep their `RuntimeError`. Every other `Core\Session`
  member already refuses without a started session (`crates/nvs-stdlib/src/session.rs:855`).
- **A child's limit breach is `ok = false` with the parent still running.** Only the depth breach is
  proven (`crates/nvs-host/tests/limits.rs:769`); a memory breach and a CPU breach inside a child are
  each one more test beside it. `rule:security/isolate-failure-is-a-value`.
- **A cyclic argument crosses a real spawn.** The walk is proven alone
  (`crates/nvs-runtime/src/graph.rs:1108`); one test drives it through `Isolate::run` and reads the
  cycle back out of the child's answer. `rule:security/isolate-values-cross-by-copy`.

## Stage 4 — the `spawn` trace event

One file set: `crates/nvs-runtime/src/ctx/trace.rs`, `crates/nvs-host/src/isolate.rs`,
`crates/nvs-host/src/group.rs`.

- **`Ctx::record_spawn`**, beside `record_query` and `record_http`
  (`crates/nvs-runtime/src/ctx/trace.rs:155`, `:181`), under `TRACE` or `PROFILE`
  (`crates/nvs-runtime/src/ctx/mod.rs:255-257`): the form, the start timestamp, the join timestamp and
  the overhead split `rule:observability/spawn-is-its-own-event` defines — the parent-observed wall time
  less the child's own. Its payload is carried the way the other kinds carry theirs, rendered, until
  `rule:testing/debug-probes`'s sink gives every kind fields (`crates/nvs-runtime/src/ctx/trace.rs:45-53`).
- **Emitted where the child is started and closed where it is joined** — `Isolate::start` and
  `Started::join` (`crates/nvs-host/src/isolate.rs:401-420`, `:674`) for `spawn script` in both its
  entry forms and on either placement, and `run_group`'s child spawn
  (`crates/nvs-host/src/group.rs:376`) for a `Core\Task` child. Nothing is recorded, and nothing is
  read from the clock, while both bits are off.
- **The three forms** the rule names map onto what the language has: `spawn` is a `Core\Task` child (a
  task on this core, same heap — `docs/decisions/0006.md:151`'s own gloss), `spawn script` is an
  isolate on this core, and `spawn worker` is an isolate placed `on: "worker"`. The event's form tag
  uses the rule's three spellings.

## Stage 5 — the record

One new record, and no other number. Its body is § *Standing decisions*' `on: worker` bullet, argued:
ADR 0006 fixes what a worker placement *means* (`docs/decisions/0006.md:151-153`, `:328-329`) and
`rule:security/isolate-values-cross-by-copy` what crosses it, and neither says **which core, how a
child is started there, how its answer comes back, or what bounds the pool** — which is the whole of
the mechanism. It creates one rule, `designed`:

| Rule | Says |
|---|---|
| `concurrency/on-worker-runs-the-child-on-another-core` | a child placed `on: "worker"` runs on a core other than its parent's; its argument and answer are copied at every node (no move across cores); it is started through a per-core inbox and answered by the reactor's cross-thread wake carrying an id; it is still its parent's child — cancelled with it, charged to its tree's budget; in `nvs serve` the core is a sibling serving core, and elsewhere a lazily started worker core, bounded at the core count |

It modifies `observability/spawn-is-its-own-event` only if stage 4's mapping of the three forms needs
saying in the rule rather than in the code.

## Stage 6 — the three options that were parsed and dropped

One file set: `crates/nvs-types/src/expr/isolate.rs`, `crates/nvs-ir/src/lower/expr.rs`,
`crates/nvs-stdlib/src/script.rs`, `crates/nvs-host/src/group.rs`, `crates/nvs-host/src/isolate.rs`, a new
module under `crates/nvs-host/src/` for the worker cores, and `.nvst` cases under
`tests/conformance/isolate/`.

- **`on: "worker"`** — M5's own scope (`docs/plan/m5.md:40`). The type check accepts `"worker"` and
  `"here"` and refuses anything else at compile time; the lowering passes it as one more argument to
  `nvs_core_script_spawn` (`crates/nvs-ir/src/lower/expr.rs:3188-3240`,
  `crates/nvs-stdlib/src/script.rs:619`); the host seam (`crates/nvs-host/src/group.rs:190-203`) routes a
  worker placement to stage 5's mechanism. A child on a worker core answers exactly as one on this
  core does — the same `ScriptResult`, the same refusals at both crossings, the same `ok = false` for
  its failure — and a cancelled parent cancels it across the thread before its own call returns.
- **`limits:` and `grants:`** — M5's scope deferred their enforcement to M6 (`docs/plan/m5.md:43-44`)
  and no goal carries it. `limits:` narrows a child's own ceilings beneath the tree's
  (`rule:security/isolate-budget-is-the-trees`: never above them); `grants:` narrows the capabilities it
  inherits and can never widen them — the same narrowing
  `tests/conformance/isolate/a-child-inherits-a-narrowed-capability-and-cannot-widen-it.nvst` proves for
  the configured overlay. The diagnostic's own help text is why this is not optional: a `grants:`
  that were silently dropped would hand the child its parent's authority.

## Stage 7 — the speedup

One file set: `benches/abi-probe/benches/isolation.rs` and `benches/abi-probe/tests/perf_guards.rs`.

- **A CPU-bound fan-out is near-linear across cores** — M5's Verify line. A static method that burns a
  fixed amount of arithmetic, fanned out as N children placed `on: "worker"` and awaited, against the
  same N run one after another on one core. The bench row goes in `isolation.rs` beside
  `isolate/spawn_to_result` (`benches/abi-probe/benches/isolation.rs:68`); the guard goes in
  `perf_guards.rs` beside `a_spawn_to_result_round_trip_stays_in_the_microsecond_class`
  (`:486`), release-only as every guard there is, **best of several interleaved rounds**, and returning
  without asserting on a machine with fewer than four logical CPUs — both for the reasons
  `crates/nvs-cli/src/serve.rs:1801-1831` gives. The floor is a named constant with the paragraph that
  justifies it: four cores do at least three times the work one does. `rule:testing/perf-two-mechanisms`'s
  per-PR guard, self-relative on one machine.

## Stage 8 — ThreadSanitizer

One file set: `.github/workflows/ci.yml`, a new `tools/tsan.sh`, `docs/agent/commands.md` § *Fuzzing
and callgrind on Windows: use WSL*, and `crates/nvs-host/src/scheduler.rs`'s stack switch.

- **A `tsan` job** beside `asan` (`.github/workflows/ci.yml:530-574`), nightly and at release under
  `rule:testing/the-deep-lane`, gated on the same `native` change output: `RUSTFLAGS=-Zsanitizer=thread`,
  `-Zbuild-std` so `std` is instrumented too (without it TSAN reports `std`'s own synchronisation as
  races), `--target x86_64-unknown-linux-gnu`, `--tests`, over `-p nvs-host -p nvs-runtime` — the two
  crates with threads in them: the blocking pool, the watchdog, the reactor's remote wake, the drain
  bit, and stage 6's worker cores. The banner at `:460-463` is rewritten to say what each of the three
  sanitizer jobs covers.
- **Runnable locally**: `tools/tsan.sh` is the job's command, run from a Windows shell as
  `wsl.exe -- bash -lc "bash tools/tsan.sh"` over the `/mnt` mount with its target directory under
  `/var/tmp` (`docs/agent/commands.md:607-611`'s reason), ending on one line the check reads. The
  paragraph beside the `leak-check.sh` one in `docs/agent/commands.md` says how, and `docs/setup.md`
  gets the nightly toolchain and `rust-src` for the distro.
- **The stack switch.** `corosensei` has no ThreadSanitizer support, and TSAN tracks one shadow stack
  per thread, so an unannotated coroutine switch reads as every task's frames belonging to one
  stack. The switch is annotated with `__tsan_create_fiber`, `__tsan_switch_to_fiber` and
  `__tsan_destroy_fiber`, compiled only under a `--cfg nvs_tsan` the leg alone passes, each call an
  `#[expect(unsafe_code, reason = …)]` site in the shape `crates/nvs-host/src/cpuclock.rs` uses.

## Stage 9 — the rulebook

Flip stage 5's rule and `observability/spawn-is-its-own-event` to `shipped`, with `guardedBy` filled
from this goal's cases and tests, and `python tools/rules.py --render`.

## Standing decisions

- **The user's standing rule for this program**: milestones M0–M8 are complete, and every promise a
  past milestone's plan made is built. An item goes to M9+ (`docs/plan/m9.md` … `m17.md`) only if it
  **cannot** be built without that milestone's work — never because it is large. Nothing in this goal
  qualifies.
- **Decide and record, never `BLOCKED` for a design call.** A call a session meets is settled under
  the priority ordering (ADR 0004: security, then PHP-compatible correctness, then request-path
  latency, then simplicity, then memory) and written where AGENTS.md says it lives — a rule fragment,
  the record of stage 5, or the crate's module doc.
- **CI is not running** (a billing block the user is fixing), so every check here runs on this
  machine. Stage 8's job is proven by a `command` check that reads the workflow file, and the
  sanitizer itself by a check that runs `tools/tsan.sh` under WSL.
- **`on: worker` is the mechanism, and the record decides only the mechanism.** What a worker
  placement means is already decided (ADR 0006, `rule:security/isolate-values-cross-by-copy`); the
  record fixes how. Settled here, for the record to argue: a task never migrates
  (`rule:concurrency/a-wake-never-moves-a-task`), so the child is *started* on the other core — its
  program, its copied argument and its parent's task id cross through a per-core inbox the reactor
  already polls, and the answer comes back as a copy in a slot plus a `RemoteWake`, the shape
  `crates/nvs-host/src/blocking.rs:14-20` already uses. Under `nvs serve` the other core is a sibling
  serving core, chosen round-robin; under `nvs run`, a job and a test, where one scheduler is all
  there is (`crates/nvs-cli/src/runner.rs:454`), worker cores are started lazily and bounded at the
  core count, as the blocking pool bounds its threads (`crates/nvs-host/src/blocking.rs:29-36`).
  **Fallback** if a sibling serving core cannot take a child without starving its own requests: `nvs
  serve` uses the lazily started worker cores too, and says so in the record.
- **`Core\Task::map` stays on its core.** Its children are closures over the caller's heap
  (`crates/nvs-host/src/group.rs:1-2`), and a closure never crosses a boundary
  (`rule:security/isolate-values-cross-by-copy`), while `{limit, deadline}` is the only options
  shape (`rule:concurrency/limit-and-deadline-are-the-only-bounds`). M5's Verify sentence names
  `Task::map`; this goal proves the speedup through the construct `docs/plan/design.md:262` gives
  CPU-bound work — a child placed on a worker core. **Settled with the user, 2026-09-13**: `map` stays
  concurrency on one core, the speedup is proven through worker-placed children, and m5.md's Verify
  sentence is corrected to say so (goal `plan-truth` owns the plan file's wording, this goal the proof).
- **`Core\Server::isDraining` answers in a child**, as it does in a CLI program: it reads the
  process's drain bit, not request state, and `crates/nvs-stdlib/src/server.rs:25-28` already decided
  a program that is not being served reads `false`. The rule's last paragraph gives `Core\Env` and
  `Core\Cli` the same exemption for the same reason. The session adds one sentence to
  `rule:security/request-state-throws-in-an-isolate`'s fragment naming `isDraining` as process state; if
  `rules.py` refuses a text change without a record, the sentence goes in
  `carried-gaps.md` instead and the fragment is left alone. The rest of
  `Core\Server` — the request's environment and `traceId()` — is `crates/nvs-stdlib/src/server.rs:11-14`'s
  known gap, not this goal's.
- **`race` is not built.** M5 never promised it: `docs/plan/m5.md:22` records it as deferred, and
  `rule:concurrency/limit-and-deadline-are-the-only-bounds` says there is no `race` and reserves the
  hedging name.
- **The 100k test is release-only**, and on Linux it demands `vm.max_map_count` of at least 262144
  rather than lowering its own count to fit — a test that proves 32k on the platform servers run on
  has not proved the claim.
- **TSAN's fiber annotations are the only new `unsafe` here**, compiled out of every build but the
  leg's. **Fallback** if an annotated switch still reports: the leg narrows to the tests that cross
  threads without switching stacks (the blocking pool, the watchdog, the reactor, the drain bit, the
  worker inbox), and the job's comment names what it skips and why.
- **What it spends** (`rule:programs/memory-priority`): the 100k test reserves 100 GiB of address space
  and is resident in what its tasks touched, for the length of one release-only test. A worker child
  costs one context and one pooled stack on the other core, plus its argument and answer copied at
  every node (the move is not available across cores) — per in-flight child, charged to its tree. The
  worker cores are at most one scheduler thread per core, started only when a program first places a
  child on one: O(cores), never O(requests). The `spawn` event costs one flag test per spawn with
  both bits off, and one event per spawn with either on.
- **ADR slots**: the one record of stage 5.
- **Not this goal**: the `callable`-in-`Task::all` sentence and the other stale prose of
  `docs/plan/m5.md` (goal `plan-truth`'s); module-doc gaps tagged `unowned` in these files (goal
  `unowned-closures`'s, after the user's decision sheet); a `spawn` or `spawn worker` keyword — the
  roster (`rule:concurrency/one-scheduler`) carries both through `Core\Task` and `spawn script … on:`;
  `Core\Cli` refusing in a child (`rule:tooling/the-tty-belongs-to-the-main-task` is `designed` and
  unscheduled); `Core\Server`'s request-reading members; making a refused stack reservation anything
  softer than a panic. A session that finds one on its path writes it to the handoff's `## Backlog`.
