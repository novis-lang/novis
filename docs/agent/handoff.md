# Handoff

## State

**Goal `test-doubles` is met.** Every `[[check]]` in its acceptance list passes, `python
tools/verify.py` is 12 of 12 and `--doc` reports every link resolving; `python tools/owners.py
--closes test-doubles` and `python tools/playbook.py --closes test-doubles` each own nothing.

**`rule:testing/doubles` is `shipped`** at `docs/rules/testing.json:140`, and its `guardedBy` names
the six `.nvst` cases and `crates/nvs-types/tests/testing.rs`.
`rule:testing/task-tree-and-virtual-clock` names the three `assert-completes-*.nvst` cases beside
the runner it already had. `docs/rules/testing.md` and `docs/ground-rules.md` are the render of
those two edits.

**Stage 6's conformance cases were already on disk** when this session opened — ADR 0079 § 10's
three bullets, `assertCalled`/`assertNeverCalled`, the `partial` delegation and the three
`assertCompletes` refusals — so only the rulebook half was left to do.
`crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt` holds `Core\BigInt` alone.

## Next group

**Goal `bigint`, stages 2 and 3 — the class and its roster** — one file set:
`crates/nvs-stdlib/src/bigint.rs` (new), `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-stdlib/src/lib.rs`, `crates/nvs-stdlib/Cargo.toml`, `Cargo.lock`. The driver installs
`docs/agent/goals/69-bigint.handoff.md:1` over this file at the goal switch; these three items are
that seed's, so a session opening before the switch is not lost.

- [ ] **The crate and the module** — `num-bigint` into `crates/nvs-stdlib/Cargo.toml`, `mod bigint;`
      beside `crates/nvs-stdlib/src/lib.rs:242`'s `mod test;`, and `crate::bigint::CLASS` beside
      `crates/nvs-stdlib/src/registry.rs:1530`'s `crate::decimal::CLASS`.
      `rule:core-api/shape-rules`.
- [ ] **The two slots and the two descriptor fields** — `crates/nvs-stdlib/src/instance.rs:185` is
      where a `Core` class's `renderer` and `comparer` are declared, and
      `crates/nvs-stdlib/src/decimal.rs:84` is the row shape to copy.
- [ ] **Every row of the goal's § *Stage 3*** — `docs/agent/goals/69-bigint.md:1` — each with its
      reference card, under conventions.md's five edits, striking
      `crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt:22` in the same edit.

## Backlog

- `Core\BigInt` is the one key left in
  `crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt:22` — goal `bigint`.
- Goal `bigint`'s stage 4 depth cases are disjoint from the group above and can take their own
  session — `docs/agent/goals/69-bigint.handoff.md:30`.
- Mutation testing is M10's, not a `Core\Test` goal's — `docs/plan/m10.md`.
