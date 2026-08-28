# Handoff

## State

**M4's Stage 8 gate is on disk and green, and the guard-name debt is closed.** The tree is at
**867 conformance plus 189 differential**. Nothing is blocked.

`crates/nvs-ir/tests/refusals.rs` is `every_refusal_is_a_diagnostic_or_decided`: it runs
`tools/holes.py` over the tree — the tree's one recognizer for a refusal site, never a second regex
— and fails on any site no open numbered item in `loop-goal.md` claims and no allowlist entry holds.
The allowlist is **empty and may never grow**; the two judgements that keep it that way are bullets
in `loop-goal.md` § *Standing decisions*, and the test's own module doc is the mechanism's home.
Attribution is by file, so the same test carries `CEILING` — **15**, the tree's current total — which
ratchets down and never up, and a session that closes a site lowers it in the same slice or the
test says so.

The 15 that stand are all claimed: item 1 holds 9 (every one an `emit.rs` catch-all arm), item 16
three, item 25 two, item 4 one. `python tools/holes.py --item N` prints any of them.

## Next group

**The `emit.rs` catch-alls, classified the way `ty.rs`'s two just were.** Each is a `_ =>` over an
IR enum that lowering itself builds, so each is either a real hole or an engine invariant that no
program reaches — and the second kind is `CodegenError::Internal` rather than
`CodegenError::Unsupported`, which is what takes it off `holes.py`'s worklist. The proof each slice
owes is the same: name the variants lowering can construct, and show the arm holds none of them.
File set: `crates/nvs-codegen/src/emit.rs`, `crates/nvs-ir/tests/refusals.rs` (the `CEILING`
constant), `crates/nvs-ir/src/ir.rs` (the enums being matched).

- [ ] **The three whole-enum arms** — `emit.rs:3074` (`the terminator {other:?}`), `emit.rs:3458`
      (`the runtime helper {other:?}`), `emit.rs:2906` (`a refcount operation on representation
      {other:?}`). `Terminator` and `Helper` are `nvs-ir`'s own enums and lowering is their only
      producer; a variant nothing constructs is an invariant, one lowering emits is a hole.
- [ ] **The three representation arms** — `emit.rs:670` (`reinterpret`), `emit.rs:711` (widening
      into a tagged value), `emit.rs:750` (narrowing out of one). ADR 0007 § 2's grid is what says
      which pairs a program can ask for.
- [ ] **The three operator arms** — `emit.rs:1204`, `emit.rs:1348`, `emit.rs:1904`. These are item
      1's actual work (`python tools/holes.py --item 1`), ADR 0007 § 4's promotion table, and the
      only ones of the nine likely to be a real hole rather than a classification.

## Backlog

- Items 16 (named/spread arguments, 3 sites) and 25 (`object`'s representation arm, 2 sites) —
  `docs/agent/loop-goal.md`, both still with their own file sets.
- Item 4, the bitwise operators, 1 site — `docs/agent/loop-goal.md`.
- `holes.py`'s attribution is by enclosing function first and by *nearest anchor in the same file*
  second, so an item's anchors are load-bearing for the Stage 8 gate now; `tools/holes.py`'s own
  docstring owns that.
