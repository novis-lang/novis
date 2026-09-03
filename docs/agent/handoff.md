# Handoff

## State

**M8 goal 5, stage 10. The 292-byte leak is closed**, and with it the driver's failing acceptance
check: `wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh examples/queue.nvs` exits 0 where it exited
97.

**It was never the database's throwing path.** `nvs_host::isolate::finish` copies a child's answer
into the *collector's* ownership root and releases it on the thrown and cancelled arms only — the
ordinary arm hands it back on `Completion.value`, whose field doc in
`crates/nvs-runtime/src/host.rs` is now the home of that obligation. `nvs queue`'s worker is the one
collector with nobody to answer, and it dropped the field: `examples/queue/receipt.nvs` ends in
`return ["receipted" => true]`, so one array literal leaked per job that returned one — growth with
jobs *served*, which the priority ordering calls a leak rather than a footprint.
`Completion::discard_value` is the discharge, and it is a safe method rather than a release at the
call site because `nvs-cli` denies `unsafe_code` and holds no `unsafe` block at all.

**The previous handoff's hunt was pointed by a misread stack, and no db fixture was written** — the
new playbook bullet owns why `leak-check.sh`'s printed frames belonged to other loss records. There
is nothing left to reproduce under `tests/db/`.

**The array copy-on-write write path is audited and clean — do not re-audit it.** Every `extern "C"`
mutator in `crates/nvs-runtime/src/array.rs` writes its handle back through `into_raw()` before it
decides on a fault, and the two that can fault are pinned by counting tests over `live_bytes` at
`crates/nvs-runtime/src/array.rs:2659` and `:2699`.

**Orientation gap, carried:** `[context]` still has no field that can name a `docs/spec/` file.

## Next group

**One file set: `crates/nvs-stdlib/src/queue.rs` and `tests/conformance/core/queue-*.nvst`**, under
ADR 0084 §§ 4–6. `python tools/gaps.py` ranks `Core\Queue\Stats` at depth 3 / floor 3 and
`Core\Queue` at 4 / 3 — the two thinnest classes inside this goal, and every member below already
has its three cases, so what these add is a boundary or an invariant and never another row of the
same shape. The four depth shapes are in `docs/agent/conventions.md`.

- [ ] **`Core\Queue\Stats`'s counters, asserted as an invariance over a sweep rather than row by
      row.** The three read the same record and must agree on what one job contributes to each:
      `crates/nvs-stdlib/src/queue.rs:2527` (`claimed`), `:2535` (`attempts`), `:2543`
      (`deadLettered`). `queue-stats-counters-agree-on-what-a-count-is.nvst` is the agreement case
      to extend from, not to restate.
- [ ] **`Core\Queue::cancel` at the bound both sides of it** — the last state a cancel wins from and
      the first it does not, named together, at `crates/nvs-stdlib/src/queue.rs:2366`.
      `queue-cancel-answers-whether-it-won-the-race.nvst` asks only the winning half.
- [ ] **`Core\Queue::status` on a receipt that names no row** — the edge, at
      `crates/nvs-stdlib/src/queue.rs:2273`. § 6 makes a dead-lettered job one `status` still
      answers about, so absence and dead-lettering are two answers and not one.

## Backlog

- The differential suite holds 250 against stage 10's `min_passing = 255`; `gaps.py` names
  `Core\Db::connect` and `Core\Db::quoteIdentifier` as the two members with a PHP twin and no oracle
  case. Oracle cases go under `tests/differential/` only (`docs/agent/conventions.md`).
- `Core\IO\Metadata` is the thinnest class in the tree at depth 1.0, and is outside goal 5.
- 121 unasserted error paths, 5 of them `thrown` rather than `fatal` — `python tools/gaps.py`.
- `[context]` in `docs/agent/loop-goal.toml` has no field that can name a `docs/spec/` file.
