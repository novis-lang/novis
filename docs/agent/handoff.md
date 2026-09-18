# Handoff

## State

Goal `types-enum-2-2` — types:enum (2/2) — is met, the floor check that held it is green again, and
`python tools/verify.py` is **13 of 13 green** (4879 tests, 2088 conformance, 279 differential,
clippy clean). All 17 enums carry a description, three examples and an attributed test.

**The `abi-probe [1 floor]` failure was the machine, not the tree.** All 21 guards in
`benches/abi-probe/tests/perf_guards.rs` pass alone, and one Rust line landed in this whole run — a
`// covers:` marker. The fan-out guard now measures what the box can give, the same children on plain
threads, beside what the placement reached, and reports itself not measured when the machine cannot
supply the cores its ratio is about. Idle it reads 3.86x of an available 3.16x against a 2x floor; on
a saturated box the placement reads 0.08x and the guard says so rather than going red.

The two load-sensitive tests the last handoff named — `crates/nvs-host/src/net.rs:2138` and
`crates/nvs-server/src/serve.rs:5379` — both passed this run.

`ArithmeticError` landed too, ahead of goal `types-exception`'s switch: a page, three blessed
examples, an attack that raises 400,000 refusals and rethrows from 20,000 frames, and a `covers:`
marker on the case that pins this class hanging off `Throwable` rather than `RuntimeError`.

## Next group

**Goal `types-enum-2-2` is met, so the driver's goal switch installs goal `types-exception`'s own
generated handoff over this one.** Its next three exceptions, in the order `TREE` declares them, so
the next session does not re-derive the group — one file set: `crates/nvs-hir/src/errors.rs`,
`docs/examples/types/<name>/` and `tests/hostile/types/<name>/`, with `rule:testing/four-proofs`
naming what each owes and `crates/nvs-codegen/tests/arithmetic.rs:40-57` the spelling a `catch`
clause and a `->message` read take.

- [ ] **`Core\Cli\NotInteractive`** — page, three examples, an attack and a `covers:` marker.
      `crates/nvs-hir/src/errors.rs:106`
- [ ] **`Core\Db\DbError`** — the same four; a `RuntimeError` subclass, so a `catch (RuntimeError …)`
      clause does catch this one. `crates/nvs-hir/src/errors.rs:107`
- [ ] **`Core\Db\RolledBack`** — the same four, and the neighbour in that file.
      `crates/nvs-hir/src/errors.rs:108`

## Backlog

- An exception owes no bench, and a hostile file is the attack — `python tools/dossier.py --id
  '<feature>'` is what says so per feature, and it is the cheapest first call of such a slice.
- `docs/examples/` holds some 130 files `nvs fmt` would rewrite. Nothing asks it to, and the
  playbook's *Writing a test case* section now says why; a sweep would be its own goal.
