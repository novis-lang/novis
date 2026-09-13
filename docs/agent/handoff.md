# Handoff

## State

**Goal `process-cache` — stage 8 is landed and every rule the goal ships is `shipped`.** The four rules
`because` ADR 0181 created — `concurrency/the-process-tier-is-one-store-per-process`,
`concurrency/a-secret-is-cached-only-sealed`, `concurrency/a-secret-fill-runs-once-per-process` and
`http-server/a-session-holds-a-secret-only-sealed` — each name their conformance cases in `guardedBy`,
and `python tools/rules.py --render` has rewritten `docs/rules/concurrency.md` and
`docs/ground-rules.md`.

**The `1 floor` rulebook check was red for a reason that was not the statuses.** A playbook bullet
carried a placeholder citation — `rule:` followed by a made-up topic and slug — and `rules.py --check`
resolves every citation under `docs/`, so the bullet that then described the trap, and the handoff that
pointed at it, re-introduced the token the moment the wrap wrote them. Both now describe the token in
words, the check is clean, and `session.py --wrap` refuses a wrap body that carries one, which is the
last moment the body is still only in the wrap file. The trap is in the playbook under *Tooling*.

**What the goal being met rests on in-session:** `python tools/rules.py --check` clean, `python
tools/verify.py` 11 of 11 green (1864 conformance, 276 differential, 4381 unit), and `python
tools/verify.py --doc` green. `python tools/loop.py --goal-only` was *not* collected — it spends its
first ten minutes on the valgrind sweep and was stopped there — so the end-to-end acceptance is the
driver's own run, not this session's.

**The `[context]` gap is unchanged:** the goal's own stage prose (`docs/agent/loop-goal.md:118-190`) is
reachable from no field, so a session meets a stage only through the header the driver prints above a
failing check. Stage 8's prose is what says the flip covers *four* rules; the handoff's group named two,
because the other two were already `shipped`.

## Next group

**The goal is met — what remains is the next goal in the chain, not this one.** The two loose ends this
goal deliberately did not take are below; both are `crates/nvs-stdlib/src/cache.rs` work.

- [ ] **A fill is single-flight per *process*, never per fleet** —
      `crates/nvs-stdlib/src/cache.rs:3265` (`fill_runs_once_while_every_other_core_waits`) is the
      guard, and a fleet-wide fill is a lease over the shared tier, which
      `rule:concurrency/a-secret-fill-runs-once-per-process` does not promise. Nothing to write until a
      goal asks for the lease.
- [ ] **`getSecret` tries every key of the ring rather than the newest alone** —
      `crates/nvs-stdlib/src/cache.rs:1672` and `crates/nvs-stdlib/src/keyring.rs:136` (`newest`), the
      per-call cost the goal's § *Standing decisions* states. A ring long enough for that to matter has
      no bound today.

## Backlog

- A `secret bytes` value — deferred by goal `process-cache` § *Standing decisions*, owned by
  `rule:security/secret-qualifier`.
- A fleet-wide single fill as a lease over the shared tier — `docs/decisions/0181.md`.
- The `nvs/rest` package, the first caller of all of this — unscheduled, `docs/agent/goals/`.
