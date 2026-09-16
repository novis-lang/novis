# Handoff

## State

**Goal `unowned-closures`, and the register is at `unowned: 17`.** `python tools/owners.py` reports
`unowned: 17`, `milestone-owned: 18`, `past-milestone: 8`, `untagged: 0`, `broken-tag: 0`,
`unreasoned: 0` and `sections outside Known gaps: 0` over 107 items; `python tools/owners.py
--deferrals` is green.

**`regex.rs` gap 3 is closed: a run of match positions converts with one cursor.**
`crate::granularity::Unit::cursor` walks a subject once for a whole run of non-decreasing byte
offsets, so `matchAll` and `replaceWith` report *k* matches over *n* bytes in O(n) rather than
O(n·k). The edge that had held it back is the cluster spanning a match's first byte: the cursor
holds that unit rather than consuming it, which is exactly what counting the prefix on its own
answers — `Cursor::index_of_byte`'s own doc is the home of that, and
`a_cursor_answers_what_counting_each_prefix_answers` sweeps every character boundary of seven
subjects in both units.

**Three of the 17 are the goal's own work, and 14 are the user's sheet.** Left: `task.rs` gap 1
(stage 4, below), `requires.rs` gap 2 (stage 3), and `graph.rs` gap 1, which no stage list names and
whose decision is open. The other 14 need answers only the user can give, so `unowned: 0` becomes a
`BLOCKED` once those three are closed, not more session work.

**The goal's stage lists number gaps as the module docs stood when they were written**, and closing
one renumbers every gap after it. `docs/agent/goals/60-unowned-closures.md` and its `loop-goal.md`
copy are corrected again here: stage 4's **Builds** loses `regex.rs` gap 3, and the M6 bullet's
`regex.rs` gap 4 — the compiled-pattern cache — is now gap 3.

## Next group

**Stage 4: the library, the tag `all`'s result carries** — one file set:
`crates/nvs-ir/src/lower/mod.rs` and `crates/nvs-stdlib/src/task.rs`.

- [ ] **`all`'s result records the *result* shape's per-slot representations** —
      `crates/nvs-ir/src/lower/mod.rs:3352-3363` builds the result with the argument's own shape
      class, because `rule:types/object-top`'s shape class is named for its field names alone and
      those match on both sides — but that descriptor's per-slot tags come from the literal, where
      every field is a closure, so `$page->count = 5` is refused with a message naming `object`
      (`crates/nvs-stdlib/src/task.rs:49`). Degrading the shared class's tags to unchecked is the
      mechanism two disagreeing literals of one shape already take; `nvs_runtime::object`'s module
      doc § *What a shape write checks* owns it.
- [ ] **A `.nvst` case writes to a field of `all`'s result and reads it back** — the refusal above is
      invisible to every case that only reads, so the case has to write one field per representation
      the literal disagreed on and count the writes that stood, not read one off a line
      (`crates/nvs-stdlib/src/task.rs:49`). Strike the gap block once it passes.

## Backlog

- `requires.rs` gap 2 — stage 3's own file set, `crates/nvs-hir/src/requires.rs`.
- `graph.rs` gap 1 — no stage list names it and its decision is open; a `BLOCKED` candidate once
  stage 4 is clear. `docs/agent/goals/60-unowned-closures.md`.
- The 14 sheet gaps need the user's answers, so the register's last step is a `BLOCKED` rather than
  a session. `docs/agent/carried-gaps.md` § *Unowned*.
