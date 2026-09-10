# Handoff

## State

**Goal `serve-runs-the-queue` — stages 2 and 3 are landed.** `nvs serve` arms `[queue] workers`
where it arms the `[[schedule]]` ticker, on the core that ticks, and every worker stops on the
process drain as well as on `Workers::stop`. Stages 4, 5 and 6 are open.

`Draining::begin` still has **no** production caller under `nvs serve` — the only one is the accept
loop's tail (`crates/nvs-server/src/serve.rs:1386`), which this command's `keep_serving` seam never
reaches, and there is no signal handler in either crate. That is what ADR 0154 § 2 describes and not
a gap this goal closes: goal `net-os-signal` lands the caller, and the predicate is here first so
that it does not read as a defect in signal handling when it arrives.

The design stays closed. [ADR 0154](../decisions/0154.md) is the whole of it and the goal's
§ *Standing decisions* holds the three things a session must not re-decide — the command keeps its
name, the workers hold `TaskRoot::Worker`, and `arm` passes no lease.

## Next group

**Stage 4 — `[queue]` joins the directive table**, which is the block's *only* missing half: a
reload that changes `workers` today is published, changes nothing and says nothing, because
`lookup` answers `None` for every key of it. One file set: `crates/nvs-config/src/directive.rs`,
`crates/nvs-config/tests/directives.rs`, `crates/nvs-config/src/snapshot.rs`.

- [ ] **The four rows** — `queue.connection`, `queue.workers`, `queue.max_attempts` and
      `queue.visibility` in `DIRECTIVES`, beside the `schedule` row at
      `crates/nvs-config/src/directive.rs:186`. `Class::System` throughout, per
      `rule:core-classes/queue-storage-is-a-table`: work a request could redirect is work a request
      could redirect into a database it was never granted. No new diagnostic — the naming mechanism
      exists and this only gives it rows to find.
- [ ] **The split** — `connection` and `workers` are `Apply::Boot` (`worker::start` reads both once,
      at the arming the slice above landed) and `max_attempts` and `visibility` are `Apply::Reload`
      (a worker reads `QueueBounds` per turn off the snapshot its context holds).
      `rule:config/reloadability-is-its-own-field` is the rule; the carry loop a `Boot` row lands in
      is `crates/nvs-config/src/snapshot.rs:258`.
- [ ] **The census** — four entries in `crates/nvs-config/tests/directives.rs:72`, whose
      `governing(...)` case at line 156 is the shape a `queue.*` key's block name is asserted with.
- [ ] **The four named cases**, which `docs/agent/loop-goal.toml`'s stage 4 check lists and which are
      the specification: the last of them asserts that a reload changing `workers` carries the
      running value forward *and* names the key, which is the two halves of `publish` at
      `crates/nvs-config/src/snapshot.rs:319` rather than one.

## Backlog

- **Stage 5** — the help line at `crates/nvs-cli/src/main.rs:259`, which says "Serve a Novis file
  over HTTP, on one core, until stopped" and now describes the least of three subsystems.
- **Stage 6** — `examples/queue.nvs`'s properties under the *served* binary, and the shutdown case.
  **Write the shutdown case with a deadline**: a stage-2 regression is a hang, and a check that
  hangs has nothing to report.
- **A `nvs serve` from this repository's own root now arms one worker**, because `nvs.toml:378`
  writes `[queue] connection = "main", workers = 1`. Nothing hangs on it — the accept loop is
  already a parked task and `parked == 0` is what ends the process — but a bench or fixture that
  boots this tree holds a `[db.main]` connection it did not before.
- **`Workers::stop` has no caller under `nvs serve`** and is not meant to: `docs/agent/carried-gaps.md`
  is where that goes if the drain ever stops being the served stop condition.
