# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
and stages 3 to 8 are complete. Nothing is blocked. The next open stage is 9, the schedule.

**`rule:routing/an-origin-is-per-mount-and-checked-at-boot` is `shipped`.** Both halves are in
`nvs serve`: `fall_back_to` folds `[app] origin` into every row that wrote none, so `Mounted::origin`
is *the* origin a mount resolved; `compiled_under` compiles each mounted entry and refuses the boot
when the unit it just compiled builds an absolute link and that row resolved no origin; and
`at_mount_origin` puts the row's origin on the isolate that answers the request, through
`nvs_host::Isolate::at_origin` and not onto the carrier. Whether a unit links absolutely is a compile
product — `nvs_types::ExprTypeTable::links_absolutely` off the resolved call sites, carried to the
boot on `nvs_runtime::routes::Routes`, which every builder in this binary now marks.

**The prefix a request reads is the one its door stripped.** `nvs_server::mount::carry` writes `""`
for the mount at `/`, matching `strip`'s own first branch; it was writing the literal `/`, which put a
second separator in front of every link a default deployment builds.

## Next group

**Stage 9: the schedule — the fleet lease, its renewal, and each entry's sub-caps** — one file set:
`crates/nvs-stdlib/src/cache/redis.rs`, `crates/nvs-server/src/schedule.rs` and
`crates/nvs-cli/src/serve.rs`.

- [ ] **The shared tier gains a set-if-absent** — `SET key token NX PX ttl` beside the existing
      `SET … PX` at `crates/nvs-stdlib/src/cache/redis.rs:162`, whose request is built at
      `crates/nvs-stdlib/src/cache/redis.rs:193`. Internal, not a `Core\Cache` row: a
      compare-and-set a program could call is new cross-request coordination and is out of this
      goal (`rule:concurrency/cross-request-state-is-explicit`). Renewal extends only while the
      token is still this host's, which is one compiled-in `EVAL`.
      `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease`. Tests:
      `a_shared_tier_lease_is_taken_by_one_of_two_clients_and_expires_after_its_ttl`,
      `a_shared_tier_lease_is_renewed_only_while_the_token_is_still_the_holders`, `-p nvs-stdlib`.
- [ ] **`nvs serve` implements `nvs_server::Leases` over it** —
      `crates/nvs-server/src/schedule.rs:310` is the trait, and
      `crates/nvs-cli/src/serve.rs:895` is the `None` that leaves every `scope = "fleet"` entry
      unarmed today. The implementation lives in `nvs-cli` because `nvs-server` names no
      `nvs-stdlib` (`crates/nvs-server/src/schedule.rs:31`). Closes that module's known gap 1.
      `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease`. Test:
      `serve_arms_a_fleet_entry_when_the_shared_tier_can_take_a_lease`, `-p nvs-cli`.
- [ ] **A lease is renewed while its run is in flight** — `crates/nvs-server/src/schedule.rs:91`'s
      known gap 3: a timer on the fire's own task, not on the ticker's.
      `rule:config/a-scheduled-run-is-a-root-isolate`. Test:
      `a_fleet_lease_is_renewed_while_its_run_is_in_flight`, `-p nvs-server`.
- [ ] **An entry's `limits` and `grants` narrow its run and never widen it** —
      `crates/nvs-server/src/schedule.rs:85`'s known gap 2, per isolate,
      `nvs_host::Isolate::narrowed_by` being what already takes room away.
      `rule:security/isolate-budget-is-the-trees`. Test:
      `a_schedule_entrys_limits_and_grants_narrow_its_run_and_never_widen_it`, `-p nvs-server`.

## Backlog

- A mount whose entry matches a different `[[app]]` block than the served entry takes the served
  block's `origin`, not its own — `crates/nvs-cli/src/serve.rs`'s `fall_back_to` doc names it; the
  general per-application gap is `crates/nvs-cli/src/serve.rs:79-90`, owned by goal `plan-truth`.
- Stage 8's "the check re-runs on reload" is vacuous while `[server]` is `Boot`-class: a reload
  cannot change a mount or its origin, which the rule fragment now says.
- The `unowned` gaps at `crates/nvs-server/src/route.rs:30`, `crates/nvs-server/src/bounds.rs:62`,
  `crates/nvs-types/src/response.rs:29` and `crates/nvs-stdlib/src/cli.rs:130` — goal
  `unowned-closures`.
- `Core\Metrics`'s three rows — goal `m8-stdlib-depth`.
