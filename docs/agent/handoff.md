# Handoff

## State

**Goal `m5-proofs` (M5). Stage 3 is closed, and stage 5 — the record — is the earliest red check.**

Stage 3's two `.nvst` cases are both on disk and both pass. The isolate half is
`tests/conformance/isolate/a-child-refuses-request-state-and-still-reads-the-process.nvst`, which the
check at `docs/agent/loop-goal.toml:9769` now names under the name its author chose; the session half is
`tests/conformance/core/session-start-where-no-request-arrived-is-a-logic-error.nvst`, new here. Stage 3's
other check needed nothing written: all three `nvs-host` test names resolve
(`crates/nvs-host/src/isolate.rs:1616`, `crates/nvs-host/tests/limits.rs:916` and `:997`).

`rule:security/request-state-throws-in-an-isolate` already carries the `Core\Server::isDraining`
sentence the goal's § *Standing decisions* asked for, so nothing was owed there. The rule the two cases
pin now has a program answering no request on both sides of the boundary: a child inside a request, and a
test that is not answering one at all, with `start` refusing the absent request before the store even
though `[session] backend` is written in the tree.

Stage 4's gap is unchanged and still in `## Backlog`. Nothing is blocked.

## Next group

**Stage 5: the record** — one file set: `docs/rules/concurrency.json`,
`docs/rules/concurrency/`, `docs/decisions/` and `docs/agent/loop-goal.toml`.

- [ ] **A worker placement running the child on another core is a rule** — stage 5's only check,
      `docs/agent/loop-goal.toml:9812`, is `python tools/rules.py --show
      concurrency/on-worker-runs-the-child-on-another-core`, and that id does not exist yet. The fragment
      goes at `docs/rules/concurrency/on-worker-runs-the-child-on-another-core.md` with its entry in
      `docs/rules/concurrency.json`; status is `designed`, which stage 9's check flips to `shipped`. It
      must not contradict `docs/rules/concurrency/a-wake-never-moves-a-task.md:1` — the child is
      *started* on the other core rather than migrated there.
- [ ] **The record behind it, and it is the goal's only ADR slot** — `because` points at a new file under
      `docs/decisions/`; `0183.md` is the highest at this commit, so re-derive the next free number from
      the directory immediately before creating it. What it argues is settled in the goal's § *Standing
      decisions* (the per-core inbox, the copied argument, the answer as a copy in a slot plus a
      `RemoteWake`, round-robin under `nvs serve` and lazily started bounded worker cores under `nvs
      run`), and the shape it points at is `crates/nvs-host/src/blocking.rs:14`.
- [ ] **Render, then confirm** — `python tools/rules.py --render` writes `docs/rules/concurrency.md`,
      `docs/ground-rules.md` and `docs/divergences.md`; none of the three is edited by hand. Then the
      stage's only check, `docs/agent/loop-goal.toml:9812`, is the `--show` above and nothing else.

## Backlog

- `rule:observability/spawn-is-its-own-event` says the child's wall time "already arrives on
  `ScriptResult`", and that shape's fields (`crates/nvs-stdlib/src/script.rs:428`) do not carry it — the
  overhead split reads it off the native `Completion` instead. Either the rule's sentence or the shape.
- `Core\Server`'s request-reading members and `traceId()` are `crates/nvs-stdlib/src/server.rs:11-14`'s
  known gap, not this goal's.
- `docs/plan/m5.md`'s stale prose, the `callable`-in-`Task::all` sentence included — goal `plan-truth`'s.
- Module-doc gaps tagged `unowned` in these files — goal `unowned-closures`'s.
