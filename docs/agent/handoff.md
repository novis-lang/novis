# Handoff

## State

**Goal `unowned-closures`, and the register is at `unowned: 16`.** `python tools/owners.py` reports
`unowned: 16`, `milestone-owned: 18`, `past-milestone: 8`, `untagged: 0`, `broken-tag: 0`,
`unreasoned: 0` and `sections outside Known gaps: 0` over 106 items; `python tools/owners.py
--deferrals` is green.

**`task.rs` gap 1 is closed: a write to a field of `Core\Task::all`'s result is checked against the
result's own tags.** The call site records the result shape's per-slot representations against the
class its field names name (`nvs_ir::lower::Lowering::record_core_result_shape`), which merges with
the argument literal's record and degrades every slot the two spell differently to unchecked —
`nvs_runtime::object`'s module doc § *What a shape write checks*, case 4, is the home. The
program-level collapse in `nvs_ir::lower::lower_file` merges the same way now instead of keeping
whichever record sorted first, so the label's tags are one answer across frames rather than one
frame's.

**Two of the 16 are the goal's own work, and 14 are the user's sheet.** Left: `requires.rs` gap 2
(stage 3, below) and `graph.rs` gap 1, which no stage list names and whose decision is open. The
other 14 need answers only the user can give, so `unowned: 0` becomes a `BLOCKED` once those two are
settled, not more session work.

**Stage 4's **Builds** row is empty and struck.** Both its items are closed — `compress.rs` has no
`# Known gaps` block at all — so `docs/agent/goals/60-unowned-closures.md` and its `loop-goal.md`
copy carry only the M6 pair and the `Decided:` list for that stage now.

## Next group

**Stage 3: the checker and the front end** — one file set: `crates/nvs-hir/src/requires.rs` and
`crates/nvs-types/src/{ctor_init.rs,lateinit.rs}`.

- [ ] **The require harvest's wildcard arms are made exhaustive** — `requires.rs` gap 2 is an
      over-approximation whose cost is a missed name, and the arms that could miss one are
      `crates/nvs-hir/src/requires.rs:703`, `:705`, `:910`, `:975`, `:1018`, `:1191` and `:1245`.
      `rule:programs/autoload` is what a missed name breaks — a class that fails to autoload — so
      the direction is always "harvest more"; the gap's own bullet says so and is what the slice
      rewrites once the arms name every variant.
- [ ] **`ctor_init.rs` gap 2 and `lateinit.rs` gap 2 — both `scan_expr` walks made exhaustive** —
      the same shape one file over, closure bodies deliberately not counted —
      `crates/nvs-types/src/ctor_init.rs:434` and `crates/nvs-types/src/lateinit.rs:298`.
      `rule:classes/definite-property-initialization` and
      `rule:classes/lateinit-read-before-write` are what each walk answers.

## Backlog

- `crates/nvs-runtime/src/graph.rs` gap 1 — the goal's own, no stage list names it, decision open.
- The 14 unowned gaps on the user's sheet — `docs/agent/carried-gaps.md` § *Unowned*.
- `regex.rs` gaps 2 and 3, `M6`-tagged and retagged to this goal in stage 0 — the goal file's stage 4.
