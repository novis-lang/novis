# Handoff

## State

**Goal `unowned-closures`, and the register is at `unowned: 18`.** `python tools/owners.py` reports
`unowned: 18`, `milestone-owned: 18`, `past-milestone: 8`, `untagged: 0`, `broken-tag: 0`,
`unreasoned: 0` and `sections outside Known gaps: 0` over 108 items; `python tools/owners.py
--deferrals` is green.

**`rule:classes/graph-copy`'s third refusal is built.** An object holding a host handle crosses
neither carrier now: the mark is `ClassDesc::holds_host_handle()`, written by
`ClassTable::set_host_handle` from `nvs-stdlib`'s `HOST_HANDLE_CLASSES` roster and read once in
`graph.rs`'s `refusable`. The roster carries every `Core` class whose slot holds a key filed by a
`Ctx::hold_*` member, plus `Core\Socket`, whose whole state is its isolate's — `instance.rs`'s roster
doc is the home of what puts a class on it.

**Four of the 18 are the goal's own work, and 14 are the user's sheet.** The goal names, under
**Builds**: `regex.rs` gap 3 and `task.rs` gap 1 (stage 4), `requires.rs` gap 2 (stage 3); the fourth
is `graph.rs` gap 1, which no stage list names and whose decision is open. The other 14 need answers
only the user can give, so `unowned: 0` becomes a `BLOCKED` once those four are closed, not more
session work.

**The goal's stage lists numbered gaps as the module docs stood when it was written**, and closing one
renumbers every gap after it. `docs/agent/goals/60-unowned-closures.md` is corrected where that had
already bitten: stage 2's **Decided** loses `graph.rs` gaps 1–2, both landed, and stage 3's **Builds**
items name `requires.rs`'s current 2 and 3. `docs/agent/loop-goal.md` is the driver's copy of that file
and was re-synced.

## Next group

**Stage 4: the library** — one file set: `crates/nvs-stdlib/src/`.

- [ ] **`matchAll` reports every match's position in O(n)** — `crates/nvs-stdlib/src/regex.rs:85`
      gap 3: `crate::granularity::Unit::index_of_byte` counts from the start of the subject once per
      match, so *k* matches over *n* bytes cost O(n·k). `rule:types/string-is-utf8` is why a position
      is counted in graphemes at all; the matches arrive in increasing byte order, so the shape is a
      cursor that counts only the gap since the previous one, and the edge that earns a case of its
      own is a cluster spanning a match boundary.
- [ ] **`all`'s result carries the call site's per-slot representations** —
      `crates/nvs-stdlib/src/task.rs:49` gap 1: the result is built with the argument's own
      descriptor, because `rule:types/object-top`'s shape class is named for its field names alone,
      and that descriptor's slot tags come from the literal, where every field is a closure. So
      `$page->count = 5` is refused naming `object`. The fix is `nvs-ir` recording the *result*
      shape's representations at the call site, `crates/nvs-ir/src/lower/mod.rs:3352-3363`.

## Backlog

- `crates/nvs-hir/src/requires.rs:86` gap 2, the name harvest's wildcard arms made exhaustive — stage
  3's only unowned item, a file set of its own; `docs/agent/goals/60-unowned-closures.md` § *Stage 3*.
- `crates/nvs-runtime/src/graph.rs:74` gap 1, a decoded `Core` instance a program cannot narrow — the
  decision is open and no stage names it; `docs/agent/carried-gaps.md` § *Unowned*.
- The 14 unowned items the goal names nowhere — `python tools/owners.py`'s § *UNOWNED* lists them, and
  answering them is the user's sheet.
