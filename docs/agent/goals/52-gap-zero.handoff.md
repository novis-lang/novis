# Handoff

## State

**Goal `gap-zero` — the gap register is derived, empty, and the index that held it is gone — has just started;
nothing of it has landed yet.** Goal `websocket-client`'s whole list is this goal's Stage 1 floor.

Three things are settled before the first session and a session does not re-decide them:

- **`tools/owners.py` already exists** — goal `gap-owners` built it over the module docs' `# Known gaps` blocks,
  with three owner kinds. This goal **widens** it and does not write a second tool. A second index
  maintained beside the first is the failure goals `carried-gaps` and `gap-owners` both exist to end.
- **`unowned` is retired as an owner kind.** Two kinds and no third: a live goal, or an uncarried
  milestone whose file under `docs/plan/` states the scope. This is the last hand-written entry on the
  chain, so an item that can be neither is a gap this goal closes — that is the whole goal.
- **`docs/agent/carried-gaps.md` is deleted in stage 7**, and the one floor check that asserts its
  existence is replaced rather than dropped. The goal prose's § *Standing decisions* is the argument for
  that single trade; a session does not have to decide whether it may.

Read the goal prose's § *What "no open gaps" means here, precisely* before stage 3. A milestone that has
not been carried is **scheduled work, not a hole**, and conflating the two is what made the roster of
110 items look alarming.

## Next group

**Stage 0 and stage 2 together** — one file set: `tools/owners.py`, `tools/playbook.py`,
`tools/chain.py`, `tools/plan.py`, `docs/agent/carried-gaps.md`. Stage 0 is minutes and stage 2 is the
keystone; taking them apart buys a second orientation for nothing.

- [ ] **`python tools/playbook.py --check`** — it prints the thirteen § *Owned* rows whose owner
      retired without closing them (`docs/agent/carried-gaps.md:48-59`, `:69`). Strike the **owner**,
      not the entry: that file's own contract says an entry leaves exactly one way.
- [ ] **Apply the rule the struck row taught** — goal `gap-owners` struck the `{timeout?: Duration}`
      row once the module doc it pointed at said the option had landed. When the register and a module
      doc disagree the module doc is the home, and a closed row leaves the file rather than being
      re-pointed. Stage 3 repeats that shape a dozen times.
- [ ] **Widen `owners.py` to read every register** — five survive stage 7: the module-doc gap items,
      `carried-refusals.md`, the four `crates/nvs-stdlib/tests/*-outstanding.txt` ratchets,
      `guard-name-debt.md`, and the playbook's `[until:]` bullets. One roster, one exit code. The
      `want` strings in `52-gap-zero.toml` **are** the specification of its output.
- [ ] **Drop the third owner kind and add `--deferrals`** — a milestone must exist under `docs/plan/`,
      be **uncarried** (`plan.py` derives the `Carried by` cell from the chain), and its own file must
      state the scope. Deferring to a carried milestone is exactly the orphaning this goal ends, so it
      fails rather than warns.
- [ ] **Two commits**: the register's own rows struck, then the tool.

## Backlog

- **Stage 3** is the re-derivation and it is the stage that decides how much of the rest there is.
  Nothing is carried in; every item is judged against the code. A row struck carries the `file:line`
  that closed it. Two items nobody had recorded are already named in the prose —
  `crates/nvs-stdlib/src/queue.rs:1671` and `crates/nvs-stdlib/src/db/stream.rs:@unstreamed`.
- **Stage 5** is the largest single thing owed and shares one file set: `crates/nvs-stdlib/src/db/`,
  `crates/nvs-stdlib/src/queue.rs`, `crates/nvs-db/src/`. It is goal `database`'s own stages 3 and 8 —
  `stream` on four more drivers, `streamAs`, `serverVersion` on all five, and the queue's SQLite
  dialect. **It opens this goal's one ADR number** and does not name it in advance.
- **Stage 6** is six independent groups, already cut by file set in the prose. Any of them is a cheap
  second slice for a session that has the crate loaded; none of them shares files with stage 5.
- **Stage 7** deletes the index and re-points seven readers **in the same slice**. A dangling pointer
  to a deleted file is the failure this goal is about, one level up; `python tools/check-links.py` is
  the proof.
- An item that can be neither closed nor honestly deferred is a **`BLOCKED` naming the item**. The run
  holds, the user schedules it, and the session presses on. Inventing an owner puts the register back
  in the state four goals have now been spent getting it out of.
- When this goal's last check goes green the driver takes goal `dossier`.
  `docs/agent/goals/` is the schedule and this does not restate it.
