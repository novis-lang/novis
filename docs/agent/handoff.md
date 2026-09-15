# Handoff

## State

**Goal `m8-db-queue` is met, and the one floor check that held its `DONE` back is green.**
`crates/nvs-stdlib/src/json.rs`'s gap 1 carried `— owner: m8-stdlib-depth` above the item's closing
`decimal` paragraph; `tools/owners.py` reads the tag off an item's last line, so the register counted
it untagged and the goal-end sweep refused it. The tag now closes the item, and
`python tools/owners.py --check --untagged-is-an-error` reports `untagged: 0` over 166 tagged gaps.

Stage 10 is as session 0003 left it: the three rules ADR 0187 created and
`rule:core-classes/db-streaming` print `shipped` with `guardedBy` filled from this goal's own cases
and tests. Nothing is blocked.

**The pack did not print `rule:security/db-pool-reset-is-a-boundary`**, which the first item below
cites — this goal's `[context] rules` names only the three `core-classes/db-*` ones. It is fixed where
it will next be paid rather than here: goal `unowned-closures`'s own `[context]` now names that rule,
`core-classes/derive-attribute`, and the two files its three open gaps live in, neither of which it
listed.

## Next group

**All of it goal `unowned-closures`'s, whose decision sheet answers each one** — the `Decided:` line
is already on disk under every item. One file set: `crates/nvs-stdlib/src/db/mod.rs` and
`crates/nvs-types/src/derive.rs`, with `crates/nvs-stdlib/src/db/pool.rs` beside the first.

- [ ] **Build `crates/nvs-stdlib/src/db/mod.rs:214`'s gap 1** — a literal naming an endpoint no
      `[db.<name>]` block describes takes `PoolBounds::DEFAULT`, and the decision on disk is that the
      defaults apply and a deployment that wants bounds writes a block
      (`rule:security/db-pool-reset-is-a-boundary`). What is left is the first-use notice, or the
      record that there is none; `crates/nvs-stdlib/src/db/pool.rs:300` is the doc both would be
      written against, and `bounds_for_settings` at `crates/nvs-stdlib/src/db/pool.rs:342` is the half
      a test drives.
- [ ] **Close `crates/nvs-stdlib/src/db/mod.rs:240`'s gap 2** — `Db\DbError` is not in spec § 10's
      error tree so it declares no `issues`, and the decision on disk is that the split stands: a
      per-column refusal stays a `ParseError`, as for `Json::decodeAs`, and no spec change follows
      (`rule:core-classes/db-one-api`). The slice is the module doc saying that as what the tree does
      rather than as a question.
- [ ] **Build `crates/nvs-types/src/derive.rs:44`'s gap 1** — `check_row_sites` gains the
      `Core\Json::decodeAs` half the decision asks for: check `T`, and `array<T>`'s element,
      statically, so both members enforce the attribute the same way and the error is earlier. That is
      a `rule:core-classes/derive-attribute` amendment, and the run-time refusal in
      `nvs_stdlib::json` is what it moves ahead of.

## Backlog

- `crates/nvs-stdlib/src/queue.rs` gaps 1–2 and `nvs-db`'s `catalog.rs`, `ddl.rs` and `schema.rs` are
  the rest of goal `unowned-closures`'s database half — each crate's own module doc.
- `crates/nvs-stdlib/src/db/mod.rs` gap 3 and `crates/nvs-stdlib/src/json.rs` gap 1 are one knot at
  two doors: what an `Instant`, an inline shape and a skipped field are on the wire — goal
  `m8-stdlib-depth`.
- `Core\Decimal`'s `divExact`/`divRound`/`allocate` and the rest of the stdlib roster — goal
  `m8-stdlib-depth` (`docs/agent/goals/`).
- 17 recorded gaps are deferred to a milestone the program has passed; `python tools/owners.py
  --check --past-is-an-error` is the run that refuses them, and nothing schedules it.
