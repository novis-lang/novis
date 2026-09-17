# Handoff

## State

**Goal `worker-placement` — Stage 2 has landed: both `spawn script` entry forms now reach a worker
core.** A process publishes one `nvs_runtime::script::SharedResolver` handle to the compiler it
already built, a core `nvs-host` starts installs it on its own thread, and `placed::crosses` no
longer asks which form the entry is. `nvs run` and `nvs serve` both publish; a `nvs check` and a
test publish none, and a path entry there still stays on the parent's core.

The one gap `python tools/owners.py --closes worker-placement` still names is
`crates/nvs-host/src/worker.rs:98`: a serving core does not register its own inbox, so a placement
under `nvs serve` starts one of the lazily started cores rather than reaching the sibling serving
core ADR 0184 § 5 decides on. That is the goal's remaining half and Stage 3's whole subject.
Nothing is blocked.

## Next group

**Stage 3: a serving core offers itself as a destination** — one file set:
`crates/nvs-host/src/worker.rs`, `crates/nvs-cli/src/serve.rs`, `crates/nvs-host/src/placed.rs`.

- [ ] **A core can register an inbox it already owns** — `crates/nvs-host/src/worker.rs:491`'s
      `WorkerCores` only ever holds cores it started itself, each with a `Worker` handle it will
      join. A serving core has its own thread and its own reactor and needs neither, so the set has
      to hold a member whose scheduler is somebody else's. ADR 0184 § 5 is the destination set;
      `rule:concurrency/on-worker-runs-the-child-on-another-core`'s last paragraph is what it
      currently says and what changes.
- [ ] **The registration happens where a serving core starts** —
      `crates/nvs-cli/src/serve.rs:655`'s `Worker::spawn(cpu, move |sched| serve_on_worker(sched,
      core))` is the one place a serving thread comes up, beside where it installs its reactor and
      its resolver. A registration is per core and is withdrawn when that core's accept loop ends,
      so it is a guard on that thread's stack rather than a static.
- [ ] **`destination_on` prefers a registered core over starting one** —
      `crates/nvs-host/src/worker.rs:769` picks from the started set and grows it at
      `:570`. Under `nvs serve` the sibling cores already exist, so growing the set there
      oversubscribes the very cores the bound protects. State what it spends per
      `rule:programs/memory-priority` where the registration lands.

## Backlog

- The `[context] modules` manifest did not name `crates/nvs-cli/src/serve.rs`, which Stage 2 had to
  edit to publish a resolver for the fleet — `docs/agent/loop-goal.toml`.
- `nvs test` and the in-process queue worker (`crates/nvs-cli/src/runner.rs:506`,
  `crates/nvs-cli/src/worker.rs:2492`) still `scoped` without publishing, so a path entry placed
  there stays home — decide with Stage 3 whether either should publish.
- No end-to-end `nvs serve` runaway test for the resource ceilings —
  `docs/agent/carried-gaps.md`.
