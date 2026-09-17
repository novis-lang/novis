# Handoff

## State

**Stage 6, the register, is green: `python tools/owners.py` reads `unowned: 0`** over 78 items, with
`untagged: 0`, `broken-tag: 0`, `unreasoned: 0`, `retired-owner: 0`, and `--deferrals` green. Stage 6
is the goal's last stage, so **the goal is not met for one reason only**, and it is not a register
one: `session.py --wrap`'s DONE sweep refused the claim because the floor's `nvs-db (four dialects)`
block names `mysql_gives_an_indexed_text_column_a_prefix_length`
(`docs/agent/loop-goal.toml:4530`) and no such test exists. It is cause 3 in
[guard-name-debt.md](guard-name-debt.md), now the one bullet in that file's § *Names owed on purpose*:
the emitter does not write a prefix length either, so this is behaviour plus its guard rather than a
rename. The floor gate runs one session in ten, which is why the driver's acceptance runs had not
reached it.

**The last two unowned items resolved the two different ways the stage allows.** The shared-store
item took a chain entry: goal `cache-shared-dial` is goal 65, between `tds-bytes` and `gap-zero` (now
66, `dossier` 67), because none of the three refusals needs a mechanism built — the secret registry
(`crates/nvs-config/src/secret.rs:161`), the one outbound TLS client, and a `Transport` enum that
already exists to say which socket a store is reached over. `tests/db/compose.yaml:293`'s `redis`
already serves TLS on `6380`, so that goal's real-store stage needs no compose edit.

**The response item was struck as a bound, because the code was ahead of the rule.**
`crates/nvs-types/src/response.rs:173-181` already refuses two *different* typed body members in one
handler and deliberately admits one member called twice; the gap and its `carried-gaps.md` bullet both
claimed `E0801` reached only `echo` beside a member. Per the goal's *Standing decisions* the code
wins: `rule:security/response-body-is-one-typed-member`'s last paragraph now states both pairs, the
corpus case gained the arm that had none, and `crates/nvs-stdlib/src/response.rs`'s prose carries the
bound in place of the item.

## Next group

**The floor's one unwritten guard** — no prose stage of this goal, so the base manifest is the right
pack. One file set: `crates/nvs-db/src/ddl.rs`, which holds the emitter and its tests both.

- [ ] **A MySQL index over a `TEXT` column carries a prefix length** — `crates/nvs-db/src/ddl.rs:125`'s
      `create_table` writes the inline clause as `` INDEX `wide_token_idx` (`token`) ``, and InnoDB
      refuses an index on a `TEXT` or `BLOB` column that names no prefix, so a `Schema` with one emits
      DDL the server rejects. `rule:core-classes/schema-plan` is what the emitter answers to, and
      MySQL's departure is already `create_table`'s own doc.
- [ ] **The emitter learns which columns need one from the same place it learns their type** —
      `crates/nvs-db/src/ddl.rs:259`'s `column_type` is what maps a `ScalarType` onto each dialect's
      spelling, so it is where `TEXT` becomes knowable to the index clause rather than a second table.
- [ ] **The guard the floor already names** — `crates/nvs-db/src/ddl.rs:1498`'s
      `mysql_declares_an_index_inside_its_create_table_having_no_if_not_exists` is the shape and the
      neighbour; `crates/nvs-db/src/ddl.rs:1140`'s `every_construct` is the shared fixture, whose
      indexed `token` column is what the new case reads. The name is fixed by
      `docs/agent/loop-goal.toml:4530` and reconciling it is the same slice
      ([guard-name-debt.md](guard-name-debt.md) § *Why a name goes stale*).

## Backlog

- Goal `cache-shared-dial`'s own seed handoff is written and its stage 2 is the dial — the file set is
  `crates/nvs-stdlib/src/cache.rs` and `crates/nvs-stdlib/src/cache/redis.rs`.
- Goal `class-scoped-types` is what the chain takes after this one, and its seed handoff already names
  its stage 2 group — nothing to re-derive at the switch.
- The eight items deferred to a milestone the program has already passed (`python tools/owners.py`,
  second section) are a finding this goal's checks do not measure; `crates/nvs-stdlib/src/html.rs:73`
  and `crates/nvs-syntax/src/lib.rs:103` are the oldest two.
- `docs/agent/carried-gaps.md` § *Unowned* still holds the entries that are not module-doc items, so
  it does not empty when `owners.py` reads zero.
