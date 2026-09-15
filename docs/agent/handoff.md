# Handoff

## State

**Goal `m8-db-queue` is met.** Stage 10's four checks pass locally: the three rules ADR 0187 created
print `shipped` with `guardedBy` filled from this goal's own cases and tests, and
`git grep -F "owner: m8-db-queue" -- crates tools` finds nothing.

`rule:core-classes/db-streaming` is flipped with them. Stage 2 removed its *Not shipped whole*
paragraph and both members now answer on all five drivers, so `designed` was the stale half of one
change; `crates/nvs-stdlib/tests/db_stream.rs` joins its `guardedBy`.

**Two fragments were corrected while being flipped**, because `shipped` claims the tree holds every
sentence in them. `rule:core-classes/a-unique-key-reads-nulls-as-distinct` said `plan` refuses where a
filtered index cannot be read back as its key; `crates/nvs-db/src/catalog.rs:44-60` is the opposite and
is what the tree does — a predicate that is not the key's own drops the index out of the read value,
and a plan proposing a name a partial index already owns fails on the server rather than in the plan.
ADR 0187 § 3 wrote that refusal as the fallback *if* the round trip could not be made to work, and
`a_filtered_unique_index_reads_back_as_the_same_key` is it working.
`rule:core-classes/server-version-is-what-the-server-said` charged a string per open connection to all
five; SQLite reads `crate::sqlite::library_version()` and holds nothing per connection.

**The two `# Known gaps` blocks shrank rather than went.** `crates/nvs-stdlib/src/db/mod.rs` gap 3 is
closed; its gap 4 keeps only the skipped field's constructor default and is re-owned to
`m8-stdlib-depth`, beside `crate::json`'s gap 1, which is the same knot at the other door.
`crates/nvs-types/src/derive.rs` gaps 1–2 are closed and its gap 3 is renumbered to 1. Renumbering
moved three in-file citations and two in other crates, one of them the text of a runtime fatal
(`crates/nvs-stdlib/src/db/row.rs:209`, which named a gap 8 that has not existed for some time).

**Three carried floor checks named tests this goal's own work deleted**, found by the wrap's `DONE`
gate rather than by the floor, which runs one session in ten; they now name the tests that hold the
same ground on five drivers. The playbook bullet is the general form.

## Next group

**The goal is met, so this names what a session in this file set takes next** — all of it goal
`unowned-closures`'s, whose decision sheet answers each one; the `Decided:` line is already on disk
under every item below. One file set: `crates/nvs-stdlib/src/db/mod.rs` and
`crates/nvs-types/src/derive.rs`.

- [ ] **Build `crates/nvs-stdlib/src/db/mod.rs:214`'s gap 1** — a literal naming an endpoint no
      `[db.<name>]` block describes gets `PoolBounds::DEFAULT`, and the decision on disk is that the
      defaults apply and a deployment that wants bounds writes a block
      (`rule:security/db-pool-reset-is-a-boundary`). What is left is the first-use notice, or the
      record that there is none.
- [ ] **Close `crates/nvs-stdlib/src/db/mod.rs:240`'s gap 2** — `Db\DbError` declares no `issues` and
      a per-column refusal is thrown as a `ParseError`; the split stands, so the work is stating it in
      `rule:core-classes/db-one-api`'s neighbourhood and striking the gap rather than changing a class.
- [ ] **Build `crates/nvs-types/src/derive.rs:44`'s gap 1** — `check_row_sites` gains the
      `Core\Json::decodeAs` half, checking `T` and `array<T>`'s element statically, which is the rule
      amendment `rule:core-classes/derive-attribute` owes.

## Backlog

- `crates/nvs-stdlib/src/queue.rs` gaps 1–2 are goal `unowned-closures`'s — `docs/agent/loop-goal.md`
  § *Standing decisions*, *Not this goal*.
- The socket leg asserts `crates/nvs-db`'s case list alone: `queue`, `db_stream` and the worker's
  case gate themselves out on a `Location::Socket`. `tools/db-matrix.py`'s `SOCKET_SUITES` says what
  a case needing a socket would change.
- A Linux host runs the socket leg natively and has never been tried; `NOVIS_DB_SOCKET_DIR` is the
  one field that has to name the same directory at both ends there.
- `crates/nvs-stdlib/src/json.rs:128`'s gap 1 (`m8-stdlib-depth`) is now cited from two doors — what
  an inline shape and an `Instant` are on the wire decides both.
