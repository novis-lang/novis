# Handoff

## State

**Goal `worker-placement` is met: every stage is green and the goal owns no gap row any more.**
Stage 3 is the destination set proven rather than built. `crates/nvs-cli/src/serve.rs:1218`'s
`offer_this_core_for_placements` is the named step `serve_on_worker` takes beside its accept loops,
and `a_serving_core_registers_its_inbox_as_it_starts` reaches it from a pinned worker, places from a
second core and reads `nvs_host::worker::cores_started()`: the placement runs on the registered core
and the set grew while this process's thread count did not (ADR 0184 § 5).

Stage 3's `nvs-host` check names
`a_registered_core_is_preferred_to_starting_one_and_is_never_its_own_destination` in both toml
copies — the test that already carried the claim, rather than the drafted name no crate declared.
`docs/agent/carried-gaps.md` owns no `worker-placement` row: both module docs had already dropped
their gap sections, so the rows were what was left. `owners.py --closes` and `playbook.py --closes`
are clean and `verify.py --doc` is green. Nothing is blocked.

## Next group

**Goal switch: `core-class-tests` opens with its own handoff** — one file set:
`docs/agent/goals/63-core-class-tests.*`.

- [ ] **Take the next goal's first item from its own handoff** — the driver installs
      `docs/agent/goals/63-core-class-tests.handoff.md:1` over this file at the switch, so nothing of
      `worker-placement` carries into it except what `docs/agent/carried-gaps.md` still holds.

## Backlog
- Nothing of this goal is left open; what survives a switch lives in
  [carried-gaps.md](carried-gaps.md).
