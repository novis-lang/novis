# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
stages 3 to 8 are complete, and stage 9 is half landed. Nothing is blocked.

**The shared tier has a set-if-absent and a renewal, and `nvs serve` holds the fleet lease over
them.** `nvs_stdlib::Lease` is the door — `SET … NX PX`, and one compiled-in `EVAL` that extends a key
only while it still carries the token that took it — on a connection of its own rather than a
request's. No `Core\Cache` row reaches either (`rule:concurrency/cross-request-state-is-explicit`).
`nvs serve`'s `FleetLease` is the `nvs_server::Leases` implementation; it is opened only when the tree
has a `scope = "fleet"` entry, and every way of not reaching the store — a refusal, an unreachable
host, a connection already answering another fire — is a logged `false` rather than a guess.

**`crates/nvs-server/src/schedule.rs`'s known gaps are renumbered.** The old gap 1, that nothing
implements `Leases`, is closed; the sub-caps are now gap 1 and the in-flight renewal gap 2.

**The two wire cases and the `nvs serve` one are proved against `tests/db/compose.yaml`'s `redis`**,
and skip on stderr where it is not running: a set-if-absent and an expiry are the server's semantics,
and a fake store agrees with whatever the client that scripted it sent.

## Next group

**Stage 9: the schedule — the renewal in flight, then each entry's sub-caps** — one file set:
`crates/nvs-server/src/schedule.rs`, with `crates/nvs-cli/src/serve.rs` for the implementor half of
the first.

- [ ] **A lease is renewed while its run is in flight** — `crates/nvs-server/src/schedule.rs:83`,
      known gap 2: a timer on the fire's own task, not on the ticker's. `Leases` has one method
      today, so this is a second one beside `take` at `crates/nvs-server/src/schedule.rs:319`;
      `nvs_stdlib::Lease::renew` is written and tested and has no caller, and
      `FleetLease::asked` at `crates/nvs-cli/src/serve.rs:1155` is already the shape that answers it,
      down to `try_borrow_mut` being what keeps a fire and the ticker off one socket.
      `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease`. Test:
      `a_fleet_lease_is_renewed_while_its_run_is_in_flight`, `-p nvs-server`.
- [ ] **An entry's `limits` and `grants` narrow its run and never widen it** —
      `crates/nvs-server/src/schedule.rs:77`, known gap 1, per isolate, with
      `nvs_host::Isolate::narrowed_by` being what already takes room away.
      `rule:security/isolate-budget-is-the-trees`. Test:
      `a_schedule_entrys_limits_and_grants_narrow_its_run_and_never_widen_it`, `-p nvs-server`.
- [ ] **The rule's status is still `designed`** — `docs/rules/config.json:1093` carries it, and the
      half that is not shipped is the renewal above. Flip it and `python tools/rules.py --render` in
      the session the first item lands in, since nothing else in the tree says the rule is only half
      true. `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease`.

## Backlog

- A mount whose entry matches a different `[[app]]` block than the served entry takes the served
  block's `origin`, not its own — `crates/nvs-cli/src/serve.rs`'s `fall_back_to` doc names it; the
  general per-application gap is `crates/nvs-cli/src/serve.rs:79-90`, owned by goal `plan-truth`.
- A fleet lease's key is `nvs:lease:` and the ticker's key with no application binding, so two
  deployments sharing one store share the lease for an entry they both name the same —
  `crates/nvs-stdlib/src/cache.rs`'s `LEASE_PREFIX` doc.
- Stage 8's "the check re-runs on reload" is vacuous while `[server]` is `Boot`-class: a reload
  cannot change a mount or its origin, which the rule fragment now says.
- The `unowned` gaps at `crates/nvs-server/src/route.rs:30`, `crates/nvs-server/src/bounds.rs:62`,
  `crates/nvs-types/src/response.rs:29` and `crates/nvs-stdlib/src/cli.rs:130` — goal
  `unowned-closures`.
- `Core\Metrics`'s three rows — goal `m8-stdlib-depth`.
