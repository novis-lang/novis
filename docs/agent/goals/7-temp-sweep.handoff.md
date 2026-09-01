# Handoff

## State

**Goal 7 — ADR 0131's temporary-directory sweep — has just started; nothing of it has landed yet.** The
parity program (goals 1–6) reached its whole acceptance list and is this goal's Stage 1 floor. The design
is fully decided:
[ADR 0131](../../adr/0131-a-temporary-directory-dies-with-its-script-and-the-sweep-never-throws.md) is
written and accepted, its spec § 14 amendment landed with it, and this goal implements it without
reopening it.

Two things every session holds, in full in the goal prose: the sweep never throws and runs after the last
user code (`onExit` on a CLI ending, `afterResponse` on a request); the orphan sweep's predicate is owner
liveness, never age, and every ambiguity resolves toward *skip*.

## Next group

**Stage 2: the owned root and the end-of-script sweep** — 0131 §§ 2–3, one file set:
`crates/nvs-runtime/src/capability.rs`, `crates/nvs-host/src/isolate.rs`,
`crates/nvs-stdlib/src/io.rs`, `crates/nvs-config/src/`.

- [ ] **`capability::temp_dir` creates under the owned root** — `[io] temp_root` (a new Boot key in
      `nvs-config`), else a private `novis` subdirectory of the platform temporary directory. Entry
      naming and the `fs.write` check are unchanged.
- [ ] **The tracked list and the sweep**: the runtime records each path `temporaryDir` answers, and
      native teardown deletes what still stands at every ending the process survives — after the
      `onExit` queue, throwing nothing, logging refusals, silent about already-gone paths.
- [ ] **The registry card rewritten in the same slice** (ADR 0117): the "removing it is the program's
      own job" paragraph is replaced by 0131's contract, and `docs/novis.md` regenerated.

## Backlog

- Stage 3 (server sweeps) and Stage 4 (`nvs tmp clean`) share the pid-liveness helper — whichever lands
  first creates it, the other reuses it; its per-platform edges are a standing decision in the goal
  prose.
- When this goal's last check goes green the driver takes goal 8 — `Core\Program::id()`,
  `docs/agent/goals/8-program-id.md`. The milestone table's order 6 is M4B, whose staged goal is
  `docs/agent/next-goal-m4b.md`.
