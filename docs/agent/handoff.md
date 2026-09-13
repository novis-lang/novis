# Handoff

## State

**Goal `process-cache` is met, and the acceptance is collected in-session this time.** `python
tools/loop.py --goal-only` reports GOAL REACHED over 774 checks in 425s, and `python tools/verify.py
--doc` is green. That collection is what the previous done-claim was missing, and it is what the driver
held on.

**The `1 floor` rulebook check is green** — `python tools/rules.py --check` exits 0. The two commits that
closed it sit at HEAD and were made by hand while the run was held, so the failure line the orientation
pack prints is older than the tree it opens on. The trap is in the playbook under *Tooling*.

**Every rule the goal ships is `shipped`**: the four `because` ADR 0181 created —
`concurrency/the-process-tier-is-one-store-per-process`, `concurrency/a-secret-is-cached-only-sealed`,
`concurrency/a-secret-fill-runs-once-per-process` and `http-server/a-session-holds-a-secret-only-sealed`
— each name their conformance cases in `guardedBy`, and the generated chapters are current.

**The `[context]` gap is unchanged:** the goal's own stage prose (`docs/agent/loop-goal.md:118-190`) is
reachable from no field, so a session meets a stage only through the header the driver prints above a
failing check.

## Next group

**The goal is met — what remains is the next goal in the chain, not this one.** Both loose ends below are
`crates/nvs-stdlib/src/cache.rs` work that this goal deliberately did not take, and neither is a defect:
each is a documented bound, not an unwritten one.

- [ ] **A fill is single-flight per *process*, never per fleet** —
      `crates/nvs-stdlib/src/cache.rs:3265` (`fill_runs_once_while_every_other_core_waits`) is the
      guard, and a fleet-wide fill is a lease over the shared tier, which the goal's fill rule does not
      promise. Nothing to write until a goal asks for the lease.
- [ ] **`getSecret` tries every key of the ring rather than the newest alone** —
      `crates/nvs-stdlib/src/cache.rs:1672` states the walk and why it is the same one
      `Core\SignedCookie` takes; `crates/nvs-stdlib/src/keyring.rs:136` (`newest`) is the alternative.
      The per-call cost is the one the goal's § *Standing decisions* already states, and a ring long
      enough for it to matter has no bound today.

## Backlog

- A `secret bytes` value — deferred by goal `process-cache` § *Standing decisions*, owned by
  `rule:security/secret-qualifier`.
- A fleet-wide single fill as a lease over the shared tier — `docs/decisions/0181.md`.
- The `nvs/rest` package, the first caller of all of this — unscheduled, `docs/agent/goals/`.
