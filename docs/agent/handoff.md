# Handoff

## State

**Goal 2, Stage 2. Item 6 is closed; only item 7, the watchdog, is left.** The reactor, the parking
stream over TCP and Unix, the stack policy, deadlines on every wait — and now the two halves of the
blocking pool. `Reactor::remote_wake` (`reactor.rs:462`) issues a `Send`, one-shot `RemoteWake`: the
ids travel in a mutex-guarded `Vec` and `mio`'s waker only ends the wait, `WAKE_TOKEN` is the one
token that is not a `TaskId`, and dropping a handle wakes the task exactly as firing it does, so a
pool thread that panicked leaves nothing parked forever. `reactor.rs`'s module doc § *A wake that
comes from another thread* is that mechanism's one home, including why the outstanding-handle count
has both ends on the core — a count the waking thread decremented has a window in which `turn`
concludes nothing can wake the task and abandons it.

`crates/nvs-host/src/blocking.rs` is ADR 0106 § 6's pool: bounded at twice the core count, threads
started only when work arrives, `blocking::run` the whole handoff, and a panic in the work resumed on
the task's own stack where the task root's containment boundary meets it. Its module doc owns all of
that. **Its `Drop` does not join, and that is not a shortcut** — a playbook bullet under *Running
things* now owns the Windows TLS-destructor deadlock that made a test hang after its body finished.

**63 tests in the crate under WSL, 59 on Windows** (`cargo clippy -p nvs-host --all-targets` is clean
under WSL too). Nothing here is `#[cfg(unix)]`, but the waker is an `eventfd` on one host and an IOCP
post on the other, so both legs were run.

**The acceptance failure on `examples/tasks.nvs` (`Core\Task` has no member named `all`) is still not
a regression** — loop-goal.md § *Stage 2* writes no `Core\Task` member until this stage is green, and
`Task::all` is Stage 4's. `nvs-host` is still outside `nvs-cli`'s dependency graph, so
`THIRD-PARTY-LICENSES.txt` still needs no regeneration.

## Next group

**The watchdog, loop-goal.md § *Stage 2* item 7 and ADR 0106 § 7.** One file set:
`crates/nvs-host/src/timer.rs`, `crates/nvs-host/src/lib.rs` (the `Worker`, `lib.rs:104`) and a new
`crates/nvs-host/src/watchdog.rs`. The item's own wording is the constraint that decides the design:
it reads *the in-flight deadline each worker already maintains*, so a heartbeat written per request
is the wrong implementation and is called out as such.

- [ ] **Publish what a worker's oldest in-flight deadline is, without adding a write to the hot
      path.** `Timers` (`timer.rs:63`) already keeps it — `Timers::next_due` (`timer.rs:95`) is the
      earliest, and `Timers::arm` (`timer.rs:77`) the only place it moves. What is missing is a
      handle another thread may read it through; the shape to copy is `Remote` in `reactor.rs:132`,
      which is an `Arc` shared with something off the core and nothing more.
- [ ] **The watchdog thread itself:** one thread for the process, reading each worker's published
      deadline and reporting a worker whose oldest request has passed it by a configured margin.
      `Worker::spawn` (`lib.rs:104`) is where a worker would register itself, and the margin is a
      compiled-in default this goal says so at the site of — `[limits]` is goal 3's.
- [ ] **A test that a stuck worker is reported and a busy one is not**, which needs a task that
      parks past its deadline without the reactor waking it; `reactor.rs:571` (`turn`) is where the
      deadline is enforced today.

## Backlog

- Stage 3 (`spawn`/`await` lowering) is next after Stage 2 — loop-goal.md § *Stage 3*.
- `blocking::run`'s answer slot is one `Arc<Mutex<Option<_>>>` per call; if a profile ever shows it,
  a one-shot cell would do — `blocking.rs`'s module doc names what it spends.
- M4's 1000-case corpus count is still the residue orders 1–4 meet as the suite grows —
  docs/implementation-plan.md.
