# Handoff

## State

**Goal `worker-placement` — a serving core is a placement destination, and stage 2 is closed.**
`crates/nvs-cli/src/serve.rs:1233` registers every serving core's inbox and holds the guard to the
end of the function, so the destination set fills with threads this process was already turning and
`WorkerCores` starts none beside them (ADR 0184 § 5). A registered core's receptionist ends with the
process's drain rather than with its inbox: it stops receiving when the drain begins and returns once
what it is holding has answered, woken by the last placement ringing the bell from its own guard
(`crates/nvs-host/src/worker.rs:1043`).

Stage 2's three `-p nvs-host` tests are `crates/nvs-host/tests/placed_path.rs` — a test binary of its
own, because the destination set is one `static` per process and a core takes the published resolver
once as it starts; that file's module doc is the one home of why. Its conformance check now names the
case the corpus actually holds. Nothing is blocked.

## Next group

**Stage 3: the destination set is proven, not just built** — one file set:
`crates/nvs-cli/src/serve.rs`, `crates/nvs-host/src/worker.rs`, `docs/agent/goals/62-worker-placement.toml`.

- [ ] **`a_serving_core_registers_its_inbox_as_it_starts`, in `nvs-cli`** — stage 3's second check
      names a test no crate declares, and the registration it is about is
      `crates/nvs-cli/src/serve.rs:1233`. What is observable without booting a socket is
      `nvs_host::worker::cores_started()` (`crates/nvs-host/src/worker.rs:795`): a process whose set
      can reach a registered core starts no thread of its own, so the assertion is a placement
      answered with `started()` still at zero. `rule:concurrency/on-worker-runs-the-child-on-another-core`
      is the rule and ADR 0184 § 5 the reasoning; the serve tests at
      `crates/nvs-cli/src/serve.rs:2944` are the nearest harness on disk.
- [ ] **Stage 3's first check names a test the tree already has** — it asks for
      `a_registered_core_is_chosen_before_one_is_started`, and
      `crates/nvs-host/src/worker.rs:1402`'s
      `a_registered_core_is_preferred_to_starting_one_and_is_never_its_own_destination` is that claim
      plus a second one. Per the playbook bullet on drafted names, grep the whole of
      `docs/agent/loop-goal.toml` for the tree's name first — if no other check claims it, the repair
      is the name in both toml copies (`docs/agent/goals/62-worker-placement.toml:11270` and the live
      one), not a second test. Same rule as above.
- [ ] **Then the goal's own end** — stage 3 at
      `docs/agent/goals/62-worker-placement.toml:11262` is the last stage the goal declares, so the
      next green sweep is a `DONE`: run
      `python tools/verify.py --doc`, then `python tools/owners.py --closes worker-placement` and
      `python tools/playbook.py --closes worker-placement`, and close what they name.

## Backlog

- A placed child is started under a fresh `Ctx` on the destination core
  (`crates/nvs-host/src/worker.rs:1054`), so the tree budget ADR 0184 § 6 charges it to does not
  cross with it; no goal owns this yet.
- `nvs_server::Draining::bit` is `pub` now, for the receptionist's ending —
  `crates/nvs-server/src/serve.rs:537` carries why.
- The queue worker's core (`crates/nvs-cli/src/worker.rs`) offers itself to nothing; whether it
  should is not decided anywhere.
