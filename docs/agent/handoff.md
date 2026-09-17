# Handoff

## State

**Goal `worker-placement` — the destination set can now hold a core it did not start.**
`crates/nvs-host/src/worker.rs` has `register_this_core`, which puts an inbox in the process's set
and spawns the same receptionist on the caller's own scheduler, plus a guard that withdraws both on
drop; `WorkerCores::pick` takes the placing core's inbox and never answers it, and a set that can
reach a registered core starts no thread beside it (ADR 0184 § 5).

`nvs serve` does not call any of it yet — that is Stage 3's remaining half and the module's one
known gap. Stage 2's acceptance check is red for a different reason: its three `-p nvs-host` test
names are not written anywhere, and session 0003 landed the mechanism without them. Nothing is
blocked.

## Next group

**Stage 3: a serving core offers itself as a destination** — one file set:
`crates/nvs-host/src/worker.rs`, `crates/nvs-cli/src/serve.rs`, `crates/nvs-host/src/placed.rs`.

- [ ] **The receptionist ends when the core drains** — `crates/nvs-host/src/worker.rs:1043`'s
      `receive` loops until its inbox is closed and parks in between, which is right for a core this
      module started and wrong for a serving one: `crates/nvs-cli/src/serve.rs:1229` ends that core
      only when `report.parked == 0`, so a parked receptionist there is a server that never exits.
      `nvs_host::reactor::wake_at_drain` is the ending this process already has;
      `rule:concurrency/on-worker-runs-the-child-on-another-core` is the rule and the playbook bullet
      under *Writing Novis itself* is the trap.
- [ ] **`serve_on_worker` registers its core** — `crates/nvs-cli/src/serve.rs:805` is where the
      reactor is installed and the last line before the per-core tasks are spawned, so the
      `nvs_host::worker::register_this_core(sched)` guard belongs there and is held to the function's
      end. ADR 0184 § 5 is the destination set; `crates/nvs-host/src/worker.rs:845` is the seam, and
      landing this is what rewrites the last paragraph of
      `rule:concurrency/on-worker-runs-the-child-on-another-core` and clears the known gap in
      `crates/nvs-host/src/worker.rs`'s module doc.
- [ ] **Stage 2's three `-p nvs-host` tests** — `crates/nvs-host/src/placed.rs:438`'s
      `both_entry_forms_cross_once_a_resolver_is_published` is the nearest thing on disk and asserts
      only `crosses`. The three names the check wants are `a_path_entry_is_placed_on_another_core`,
      `a_placed_path_child_answers_what_a_same_core_child_answers` and
      `a_placed_path_no_resolver_can_compile_fails_as_a_value`. **The hazard is the core set:**
      `crate::worker`'s is one `static` per *process*, a core takes the published resolver once as it
      starts (`crates/nvs-host/src/worker.rs:671`), and the sibling tests in `worker.rs` place work
      with nothing published — so a path entry can land on a core started earlier with no resolver
      and answer `NoResolver` at random. Either publish before the set can have grown, or write them
      as an integration test under `crates/nvs-host/tests/` with a process of their own.

## Backlog

- `crates/nvs-host/src/worker.rs`'s known gap is the one thing `python tools/owners.py --closes
  worker-placement` still names; item 2 above closes it.
- A registered core on a single-CPU host is the only core there is, so a placement falls through to
  `on: "here"` — `WorkerCores::pick`'s own doc is the home, and nothing measures it.
- Stage 2's `nvs-suite` check names
  `tests/conformance/isolate/a-path-entry-on-a-worker-core-answers-as-one-on-this-core-does.nvst`;
  the tree holds `a-path-entry-placed-on-a-worker-core-answers-as-one-here-does.nvst`. One of the two
  names is wrong and the goal file is where it is fixed.
- `docs/decisions/0184.md` § *Revisiting* is the fallback the module doc cites; nobody has read it
  this goal — add it to `[context] adrs` if item 2 needs it.
