# Handoff

## State

**The floor's `nvs-db (four dialects)` block is whole.**
`mysql_gives_an_indexed_text_column_a_prefix_length` (`docs/agent/loop-goal.toml:4530`) is written at
`crates/nvs-db/src/ddl.rs:1595` and green, which was the one reason the goal was not met. Stage 6's
register is untouched and still reads `unowned: 0` over 78 items with every other
`python tools/owners.py` counter at zero.

**The guard needed behaviour, and the behaviour is narrower than the item read it.** `Table::index`
has refused an unbounded text or bytes column since `crates/nvs-db/src/schema.rs:684`, so a
`LONGTEXT` never reaches a key at all; what does reach one is a **bounded** column wider than
InnoDB's 3072-byte key budget, `text(2000)` being 8000 bytes of `utf8mb4`.
`crates/nvs-db/src/ddl.rs:509`'s `index_columns` measures each column of an index against an equal
share of that budget and writes the prefix that fits — characters for text, bytes for binary — on
MySQL alone. A prefix length `key_columns` once wrote was deleted in `72bc49fb1` for having no
reachable caller; this is the case that commit did not have.

**One gap fell out of it and is registered rather than carried here.** A unique or primary key over
such a column takes no prefix, because that would enforce something stricter than the schema asked
for, so it is emitted whole and MySQL refuses it and no grade says so. Owner `gap-zero` in
[carried-gaps.md](carried-gaps.md) § *Owned*, detail in `crates/nvs-db/src/ddl.rs` § *An engine's own
limit is the dialect's rule*.

## Next group

**The key budget the grader does not read** — no prose stage of this goal, so the base manifest is
the right pack. One file set: `crates/nvs-db/src/ddl.rs`, which holds the emitter, the grader and
their tests.

- [ ] **A MySQL unique or primary key over an over-budget column is graded as a refusal** —
      `crates/nvs-db/src/ddl.rs:867`'s `add_key` and `crates/nvs-db/src/ddl.rs:139`'s `create_table`
      both emit it whole, which `rule:core-classes/schema-plan` requires of every step, and that
      rule's § 5 grading is where a statement the server will not take is meant to become a refusal
      rather than a failure. `crates/nvs-db/src/ddl.rs:509`'s `index_columns` already holds the
      budget arithmetic the grade needs.
- [ ] **The guard for it** — `crates/nvs-db/src/ddl.rs:1595`'s
      `mysql_gives_an_indexed_text_column_a_prefix_length` is the neighbour and the shape, and the
      grade is read off a `Step` the way the floor's
      `every_step_carries_terminated_executable_sql_including_the_ones_apply_refuses` already reads
      one.

## Backlog

- The `Bytes` half of the budget is pinned at one width only; a `bytes(3073)` boundary case has no
  guard — `crates/nvs-db/src/ddl.rs:540`.
- PostgreSQL bounds a btree key too, but it refuses at insert rather than at DDL, so the emitter owes
  nothing — recorded here so it is not rediscovered as a gap.
