# Handoff

## State

Goal `core-db-transaction-and-1-more` is met **and its floor is green now**. The check that failed
session 0003's DONE claim was `dossier: lang:types`, red because every one of that chapter's 18
features carried a stale perf figure; the 18 are re-measured, and
`benches/members/lang/types/void-never-self-static.nvs` declares the one allocation a round of
record types now costs, where it had declared two.

**A SQLite write reports the key of a row it inserted itself**, which closes the finding the goal
above recorded rather than fixed. `sqlite3_update_hook` is what tells an insert from the statements
after it, and `crates/nvs-db/src/sqlite.rs`'s doc § *Which statement inserted a row* is the whole
decision: what it costs per changed row, and why neither the statement's kind nor a
`sqlite3_set_last_insert_rowid` sentinel was taken. `SqliteRows::last_insert_id` answers
`Option<i64>`, the `# Known gaps` section that held the finding is gone, and
`docs/examples/core/Db-Write/lastId/02-a-statement-that-inserted-nothing.nvs` carries no
`known-gap` marker any more. A row a trigger inserted counts as its statement's, which is the one
answer this driver gives that the connection's own value does not.

`python tools/verify.py` is green over the whole tree, and `target/release/nvs.exe` is current with
it. Nothing is uncommitted here that this session did not write.

## Next group

**Stage: the key a write reports, on the drivers that are not SQLite** — one file set:
`crates/nvs-db/src/mysql.rs`, `crates/nvs-stdlib/src/db/execute.rs`,
`crates/nvs-stdlib/src/db/registry.rs`.

- [ ] **Keep MySQL's absent key an absence rather than routing it through `0`** — the OK packet's
      id is collapsed to `0` at `crates/nvs-db/src/mysql.rs:1363` and filtered back out at
      `crates/nvs-stdlib/src/db/execute.rs:831`, so a statement that generated no key and a row
      whose key really is `0` are one answer on that driver. SQLite keeps the two apart now, and the
      member may not mean two things on two drivers. `rule:core-classes/db-statement-members`.
- [ ] **Say on the card what `lastId` answers for a statement that inserted nothing** —
      `WRITE_LAST_ID_DOC` at `crates/nvs-stdlib/src/db/registry.rs:2929` describes the PostgreSQL
      and SQL Server halves and not this one, and `WRITE_CHANGED_DOC` below it owes the sentence
      about SQLite reporting a count for every statement. Both ride in one edit because touching
      that file re-stales the perf record of every `Core\Db` member, and one edit pays that once.
- [ ] **Re-measure `Core\Db` once, after that edit** — every member's figure is keyed on
      `crates/nvs-stdlib/src/db/registry.rs:1504`'s own file, so `python tools/dossier.py
      --record-perf --group 'Core\Db\Write'` and its siblings run on an idle machine before the
      wrap, or the next session's floor opens on a group owing figures it already has.

## Backlog

- The three `Core\Db\Write` member benches measure an accessor loop over one finished write
  (`benches/members/core/Db-Write/lastId.nvs`), so the per-row cost the update hook adds to every
  write is measured nowhere. `rule:testing/member-perf-ledger`.
- An integer literal in a `??` arm widens nothing, so `?uint ?? 0` is `uint|int` — the playbook
  bullet is the workaround, and whether the literal should take the other arm's type is
  `rule:types/` territory nobody has opened.
