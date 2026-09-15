# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
stages 3 to 8 are complete, and stage 9 has one item left. Nothing is blocked.

**A `fleet` fire now holds its lease for as long as it runs.** `Leases` has a `renew` beside `take`;
`took_the_lease` hands the fire the key it took, and `Renewal::keep` spawns a timer on the **fire's
own task** that asks for the interval again at half of it. That timer is a child of the fire, so the
run ending — or § 6's `kill` cancelling it — is the whole of what stops it, and there is no guard to
write. A key this host has lost stops the timer and is one line an operator sees; the run is left
alone, which is § 3's at-most-once bound rather than a hole in it.

**`nvs serve` answers it over `nvs_stdlib::Lease::renew`**, through the same `FleetLease::asked`
funnel as `take`, so an unreachable store or a socket already answering another fire is a logged
`false`. The store is an `Rc<dyn Leases>` from the boot down, because the renewal outlives both the
`arm` frame and the tick's.

**`rule:config/a-fleet-entry-fires-at-most-once-under-a-lease` is `shipped`** — the renewal was its
last unbuilt half. `crates/nvs-server/src/schedule.rs`'s `# Known gaps` is back to one entry.

## Next group

**Stage 9: the schedule — each entry's sub-caps** — one file set: `crates/nvs-server/src/schedule.rs`,
with `crates/nvs-config/src/value.rs` for the one spelling it needs.

- [ ] **An entry's `limits` and `grants` narrow its run and never widen it** —
      `crates/nvs-server/src/schedule.rs:80`, the module's one known gap. `Armed` carries an
      `nvs_runtime::host::Narrowing` built beside `overlap` in `arm`
      (`crates/nvs-server/src/schedule.rs:404`), and `fire` at
      `crates/nvs-server/src/schedule.rs:673` applies it with `nvs_host::Isolate::narrowed_by`
      (`crates/nvs-host/src/isolate.rs:221`) — the ticker's question like `overlap`, not the
      implementor's. `Narrowing` is `crates/nvs-runtime/src/host.rs:371`: `limits` is the bare
      directive name paired with the value **as written** — `("memory", "1M")` — and `grants` is
      `Cap::name()` spellings, which `Cap::ALL` filtered by `Cap::grant`
      (`crates/nvs-config/src/capability.rs:428`) answers off the entry's block.
      `crates/nvs-runtime/tests/tree_budget.rs:259` is the shape a case reads the result back in,
      down to `Ctx::memory_limit` and `Ctx::grants_allow`; note that
      `crates/nvs-runtime/src/ctx/isolate.rs:591` applies a sub-cap only where the child's context
      carries a configuration, so a fire built on a bare `Ctx` narrows grants and nothing else.
      `rule:config/a-schedule-entry-narrows-only`. Test:
      `a_schedule_entrys_limits_and_grants_narrow_its_run_and_never_widen_it`, `-p nvs-server`.
- [ ] **`Setting` has no public rendering to the spelling a sub-cap is carried in** —
      `crates/nvs-config/src/value.rs:398`'s `as_written` is exactly it and is `pub(crate)`. Widen
      it rather than growing a second renderer in the ticker, for the reason
      `crates/nvs-runtime/src/host.rs:355` gives for carrying text at all: one reader for the number
      a file writes and the number a sub-cap writes.
- [ ] **The scope half of `grants` has nowhere to go, and the rule's fragment still says so** —
      `docs/rules/config/a-schedule-entry-narrows-only.md:8`. `Narrowing`'s grant list is capability
      *names* and carries no scopes (`crates/nvs-runtime/src/ctx/isolate.rs:624`), so
      `grants = {net.connect = ["reports.internal"]}` narrows the run to `net.connect` and leaves
      the deployment's host list standing — inside the rule and short of it. Decide it with the
      slice above: the fragment's last paragraph and one known gap, or a second channel.

## Backlog

- A fleet entry whose fire is held by `queue` or started by `kill` runs under no lease at all —
  `crates/nvs-server/src/schedule.rs:534`'s walk answers § 6 before § 3 asks, so this host had
  already lost the interval. If it should take the key instead, that is § 3's reading and belongs
  in the rule.
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
