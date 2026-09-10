# Handoff

## State

**Milestone M8, goal `queue-purge`. Stage 4 is landed:** `E0637` (`E_UNGRANTED_QUEUE`) is declared at
`crates/nvs-diagnostics/src/lib.rs:1820`, the intrinsic table reads a written `Core\Queue::purge`
queue name against the compiling machine's `queue.purge` grant, and four `-p nvs-types` tests hold it
to `E0618`'s conditions. Stages 3 and 5 were already on disk — the three
`core_queue_declares_…`/`…_reference_card_…` tests stage 5 names live in `crates/nvs-stdlib/src/queue.rs:4147`.

**The goal's prose said `E0635`, which is `E_NO_UNIX_TRANSPORT`.** The four check names, the check's
own name, the stage comment and both goal `.md` copies now say `E0637`.
[ADR 0153](../decisions/0153.md) § *Diagnostics* still says `E0635`: a record is frozen rationale, so
the goal files carry the corrected number and the record is history.

**The two goal files had drifted** — the live `docs/agent/loop-goal.toml` held `[context]` widenings
`docs/agent/goals/34-queue-purge.toml` never got. They are byte-identical again.

**Open:** stage 6's fixture alone. `python tools/reference.py --check` (stage 6's reference check)
already passes.

## Next group

**Stage 6: the fixture, and the grant a deployment writes for it** — one file set:
`examples/queue-purge.nvs`, `nvs.toml`, `docs/agent/loop-goal.toml`.

- [ ] **Write `examples/queue-purge.nvs`**, printing the eight lines frozen at
      `docs/agent/loop-goal.toml:7344` in that order — `tagged=3`, `purged-pending=2`,
      `dead-survives=1`, `purged-dead=1`, `claimed-delete=false`, `terminal-delete=true`,
      `bounded=1`, `remaining=0`. That list is the specification and the properties it pins are
      `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s: `Dead` and `Pending` are opt-in,
      a claimed row is not removable, and `limit` bounds one call. `examples/db.nvs:20` is the
      example that owns the same shape — its own comment says the connection is named in root-owned
      config and the frozen lines live in the goal file, not in the source.
- [ ] **Grant the fixture its queue**, as an `[[app]]` entry in the root `nvs.toml` beside the four
      `[app.capabilities.db]` ones (line 196), with `[app.capabilities.queue] purge` naming exactly
      the queue the fixture pushes to — `rule:security/capability-roster-is-closed`, and
      `crates/nvs-config/src/tree.rs:536` is the block's own field, one `purge` key and nothing
      else. That file's `[queue] connection = "main"` (line 359) resolves to the Postgres container
      (line 278), so this leg needs a reachable Docker daemon and the check is red without one.
- [ ] **Read the run back once** with `target/debug/nvs.exe run examples/queue-purge.nvs`, then fix
      the source and never the `want` list at `docs/agent/loop-goal.toml:7345`: the eight lines are
      the goal's, and a fixture edited to match what it printed pins nothing.

## Backlog

- The two stage 6 `nvs-suite` checks — the conformance and differential trees,
  `docs/agent/loop-goal.toml:7363`.
- `purge`'s server-side case is `purge_removes_what_has_finished_within_its_filters_and_stops_at_its_limit`
  in `crates/nvs-stdlib/tests/queue.rs` — container-gated, so `verify.py` does not run it.
- ADR 0153 § *Diagnostics* names `E0635` where the tree has `E0637`; frozen on purpose, do not "fix" it.
- Carried gaps that outlive this goal are in `docs/agent/carried-gaps.md`.
