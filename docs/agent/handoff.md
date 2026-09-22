# Handoff

## State

Goal `core-db-transaction-and-1-more` is met: `python tools/dossier.py --verify --group
'Core\Db\Transaction'` and `--group 'Core\Db\Write'` are both green, so all eleven members carry
their feature proofs. `Core\Db\Write::changed` and `Core\Db\Write::lastId` landed this session.

**`changed` is present for every statement SQLite runs, and that is now decided.** § 4's absent case
is a command tag carrying no count, and this driver sends no tag at all: a `create table` cannot be
told apart here from an `update` that matched nothing, so both report `0` and neither reports an
absence. `crates/nvs-stdlib/src/db/execute.rs:@sqlite_write`'s doc is where that is written down, and
the `.nvst` case pins it from a program.

**`lastId` found a real bug, and it is recorded rather than fixed.** On SQLite a statement that
inserted no row answers the key of the last insert on the *connection* — after an insert of row 4, an
`update`, a `delete` and a `create table` each answer `4` — which is exactly the
`mysqli_insert_id` hazard `WRITE_LAST_ID_DOC` promises this class does not have. `step`'s
`sqlite3_total_changes` fold cannot reach it, because rows changing is not rows being *inserted*, and
the three ways to tell an insert from the other kinds each cost a decision this goal may not make.
`crates/nvs-db/src/sqlite.rs:65` is the `# Known gaps` entry, owned by M10, and
`docs/examples/core/Db-Write/lastId/02-a-statement-that-inserted-nothing.nvs` is the proof marked
`known-gap` for it. The half of the promise that does hold — a write keeps its own key while later
statements run — is pinned by the `.nvst` case.

`Core\Db\Write::changed` measures 47.2 ns/op and `lastId` 40.7 ns/op, both at 0.00 allocations, as
their benches declare. `target/release/nvs.exe` is current with the tree.

**`python tools/verify.py` was green through build, fmt, test and both `.nvst` trees, and red at
clippy on files this session did not touch**: four crates carried uncommitted `eprintln!("TRACE …")`
lines somebody was debugging with, and `clippy::print_stderr` is `-D warnings`. Those lines have
since gone from the tree, and clippy was not re-run after they did. What is uncommitted now is
`crates/nvs-host/src/reactor.rs`'s `REMOTE_WAKE_BOUND` — somebody's in-flight work on a Windows
completion packet that does not release a parked core, and nothing here staged or edited it.
`python tools/verify.py --doc` is green.

## Next group

**Stage: the key a SQLite write reports** — one file set: `crates/nvs-db/src/sqlite.rs`,
`crates/nvs-stdlib/src/db/execute.rs`, and the marked proof under
`docs/examples/core/Db-Write/lastId/`. This is the goal above's finding, and it needs the decision
that goal was not allowed to make.

- [ ] **Decide how this driver tells an insert from the statements that follow it** — the three
      candidates are in the gap entry at `crates/nvs-db/src/sqlite.rs:65`: the statement's kind,
      an update hook firing per changed row, or `sqlite3_set_last_insert_rowid` as a sentinel around
      every statement, which is raw FFI through `rusqlite`'s handle. Weigh the per-row cost against
      this crate's audit surface, and record it. `rule:core-classes/db-statement-members`.
- [ ] **Report the key off the statement rather than the connection** — the read is
      `crates/nvs-db/src/sqlite.rs:1308`, and `crates/nvs-stdlib/src/db/execute.rs:982` is where the
      `?uint` is built from it. `step`'s `affected` fold two lines above is the shape.
      `rule:core-classes/db-statement-members`.
- [ ] **Unmark the proof and pin the corrected behaviour** — the `known-gap` line in
      `docs/examples/core/Db-Write/lastId/02-a-statement-that-inserted-nothing.nvs:4` goes, and the
      claim joins `tests/conformance/core/db-write-lastid-is-the-key-of-the-row-its-own-statement-inserted.nvst`.
      A marked proof that passes fails the sweep — `rule:testing/a-failing-proof-is-fixed-or-recorded`.

## Backlog

- `WRITE_CHANGED_DOC` says nothing about SQLite reporting a count for every statement; the sentence
  belongs on the card, and editing `crates/nvs-stdlib/src/db/registry.rs` re-stales the perf record
  of every `Core\Db` member, so it rides along with the next slice that has to touch that file.
- An integer literal in a `??` arm widens nothing, so `?uint ?? 0` is `uint|int` — the playbook
  bullet above is the workaround, and whether the literal should take the other arm's type is
  `rule:types/` territory nobody has opened.
