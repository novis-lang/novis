# Handoff

## State

**Goal `bigint` — `Core\BigInt` — has just started; nothing of it has landed yet.** Goal
`test-doubles`'s whole list is this goal's Stage 1 floor.

Settled before the first session: ADR 0054 § 5 names the class and the crate, the goal prose's
§ *Stage 3* is the roster and § *Standing decisions* the representation, the refusal classes and the
two things that are not this goal. The `§13 Core\BigInt` key already names this goal
(`crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt:22`), re-owned by hand on
2026-09-18.

## Next group

**Stages 2 and 3, the class and its roster** — one file set: `crates/nvs-stdlib/src/bigint.rs` (new),
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-stdlib/src/lib.rs`, `crates/nvs-stdlib/Cargo.toml`,
`Cargo.lock`.

- [ ] **The crate and the module** — `num-bigint` into `crates/nvs-stdlib/Cargo.toml`, `mod bigint;`
      beside `crates/nvs-stdlib/src/lib.rs:242`'s `mod test;`, and `crate::bigint::CLASS` beside
      `crates/nvs-stdlib/src/registry.rs:1530`'s `crate::decimal::CLASS`.
- [ ] **The two slots and the two descriptor fields** — `crates/nvs-stdlib/src/instance.rs:185` is
      where a `Core` class's `renderer` and `comparer` are declared; `crates/nvs-stdlib/src/decimal.rs:84`
      is the row shape to copy.
- [ ] **Every row of § *Stage 3***, each with its reference card, under conventions.md's five edits.
      Strike the key in the same edit.
- [ ] **`examples/bigint.nvs`** with the frozen output the acceptance list names, and the
      `tier-roster` paragraph rewritten (stage 0), then `python tools/rules.py --render`.

## Backlog

- **Stage 4, the depth cases** — `tests/conformance/core/`, four cases in the depth shapes and the
  named guard tests in `crates/nvs-stdlib/tests/`. Disjoint from the group above; its own session
  is fine.
- When this goal's last check goes green the driver takes goal `gap-zero`.
