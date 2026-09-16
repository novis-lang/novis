# Handoff

## State

**Goal `unowned-closures`, stage 6 is done, and the register is at `unowned: 19`.** `python
tools/owners.py` reports `unowned: 19`, `milestone-owned: 18`, `past-milestone: 8`, `untagged: 0`,
`broken-tag: 0`, `unreasoned: 0` and `sections outside Known gaps: 0` over 109 items;
`python tools/owners.py --deferrals` is green with M10 carrying 14 of them.

**The three retags landed.** `crates/nvs-lsp/src/hints.rs` gap 1 is `M10`, and `docs/plan/m10.md`'s lead
states the scope itself — the walk annotates `nvs_types::ExprInfo::Call` alone, and the widening is a
table question the milestone pays for on the compile path. `DEBUG_BREAK` is now a gap of its own in both
crates, `crates/nvs-ir/src/lib.rs` gap 19 and `crates/nvs-runtime/src/lib.rs` gap 8, leaving `COLLECT`
alone under gap 14 and gap 5 with the collector decision those two carry.

**The driver's red floor check is green.** `python tools/playbook.py --check` had named
`docs/agent/carried-gaps.md`'s `Core\Process::spawn` row for three sessions; the member is registered and
answers a handle (`crates/nvs-stdlib/src/process.rs:152`, pinned by
`process_spawn_is_a_registered_member`), so the row is struck rather than re-owned.

**Five items the goal's own stages name stand between here and `unowned: 0`** — `graph.rs` gaps 1–2
(stage 2), `requires.rs` gap 2 (stage 3), `regex.rs` gap 3 and `task.rs` gap 1 (stage 4) — plus the 14 the
goal names nowhere, which is the user's sheet and not a session's.

## Next group

**Stage 2: the lowering and the runtime** — one file set: `crates/nvs-runtime/src/graph.rs` and
`docs/agent/carried-gaps.md`.

- [ ] **Refuse an object holding a host handle where a graph is copied** —
      `crates/nvs-runtime/src/graph.rs:61` gap 1, the third of `rule:classes/graph-copy`'s refusals and
      priority 1: the copied key indexes the *receiving* side's table, so the child reads whatever that
      side opened at that index. The goal's stage 2 lists it under **Decided** and the module carries no
      `Decided:` sentence, while `docs/agent/carried-gaps.md:93` still frames the choice as open — a bit
      on the `ClassDesc`, the way `ClassDesc::is_closure()` marks a closure, or the declared type at the
      copy site. If neither option is written down anywhere, that is the `BLOCKED` for the user.
- [ ] **Re-point § *Unowned*'s graph.rs bullets** — `docs/agent/carried-gaps.md:79`, `:86` and `:93` name
      gaps 1, 2 and 3 while `crates/nvs-runtime/src/graph.rs` holds two: the closure-recognition bullet's
      gap is gone from the module, and the host-handle bullet is the module's gap 1. Strike what closed
      and re-point the rest in the slice above.
- [ ] **`crates/nvs-runtime/src/graph.rs:77` gap 2**, the decoded `Core` instance a program cannot narrow.
      Both refusals are `nvs-types`' (`E0496` and `E0711`, `rule:types/conversion` tabulating no
      conversion into a `Core` class), so decide whether the build belongs in stage 3's file set before
      opening it here.

## Backlog

- The 14 unowned items no stage names — the user's sheet; each reason is in `docs/agent/carried-gaps.md` § *Unowned*.
- `crates/nvs-hir/src/requires.rs` gap 2: `owners.py` counts the name-harvest over-approximation, which reads as a bound, while stage 3 describes a double-quoted cooker — settle which item the stage means.
- `crates/nvs-stdlib/src/regex.rs` gap 3 and `crates/nvs-stdlib/src/task.rs` gap 1, stage 4's two builds.
- The 8 `past-milestone` deferrals, which `owners.py` says are owed by a goal or nobody.
- `crates/nvs-lsp/src/index.rs` carries no `# Known gaps` block and `completion.rs:191` is a bold run already tagged `M10`; stage 6's text names both, so neither is work.
