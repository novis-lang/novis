# Handoff

## State

Goal `core-db-transaction-and-1-more` is **met**. `Core\Db\Transaction` (8 features) and
`Core\Db\Write` (3) both verify green — gate, examples and attacks — `owners.py --closes` and
`playbook.py --closes` name nothing for it, `python tools/verify.py` is 14 of 14 green and `--doc`
resolves every link. No perf figure anywhere in the tree is stale: a figure is keyed on the
*implementing* file's text with `mod tests` cut off, and a `Core` member's implementing file is its
registry row's, so a change under `db/execute.rs` re-measures nothing.

The floor's rulebook check was red for something this goal never touched. `agent.rs`'s citation
stripper spelled its own example with a topic and a name, and `tools/rules.py --check` reads such a
token as a citation wherever it stands — the playbook bullet above owns the repair.

**What a caller reads for `lastId` is now pinned on the two drivers nobody had pinned.** The
`Core\Db\Write` instance is built by `write_object` rather than by a literal inside the member, and
the SQL Server arm reports `Written::keyless`, so both are things a case can build without a
connection — this crate can build no `nvs_db::Connection`. Both cases read the slots back through
`write_count`, the body all three of the class's readers share.

## Next group

**Goal `core-debug`, whose generated handoff replaces this file the moment the driver switches** —
one file set: `crates/nvs-stdlib/src/debug.rs`. Take them from that goal's own brief, not from here;
these two are its first group, with the anchors it names.

- [ ] **`Core\Debug::dump` owes every feature proof** — `about.md`, tests from both sides, three
      examples, one bench, one attack. `rule:testing/feature-proofs`, at
      `crates/nvs-stdlib/src/debug.rs:107`.
- [ ] **`Core\Debug::render` owes every feature proof**, and shares `dump`'s file and most of its
      shapes. `rule:testing/feature-proofs`, at `crates/nvs-stdlib/src/debug.rs:116`.

## Backlog

- `postgres_write` reads the key only after it has drained the rows, and nothing asserts that order
  from this side: `nvs-stdlib` can build no `PgConn`. The driver's half is pinned in `nvs_db::pg`.
- A buffered read is not held to the request's memory ceiling — known gap 1 of `nvs_stdlib::db`
  (`crates/nvs-stdlib/src/db/mod.rs:286`), deferred to a milestone ahead and marked on the attack
  that found it.
- `Core\Db\Write::lastId` is proven from Rust on four drivers by unit case and by SQLite's real
  engine; the live-server half over PostgreSQL, MySQL, MariaDB and SQL Server is
  `tools/db-matrix.py`'s and runs nowhere in the floor.
