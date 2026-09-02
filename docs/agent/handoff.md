# Handoff

## State

**`Core\Db\ColumnType` is registered** — spec § 18's fourteen cases, in the spec's own order and with
their ordinals written out, at `crates/nvs-stdlib/src/db.rs:637` beside `ISOLATION` and under the same
two-halves rule: `nvs_db::ColumnType` is authoritative and this table is what a program matches on. Its
`EnumDoc` is where the value/description split now reads for a program's author — a case says what the
column was *declared* as, never what a read of it produces — so `ROWS`' doc points at it instead of
restating the three places ADR 0067 § 9 disagrees.

**What is left of gap 5 is `Core\Db\Column` and the slot, and those are one slice rather than two.**
The coverage gate matches a case's *source text*, so a class registered before anything can produce an
instance has three uncovered members and sits under the floor of three; the new playbook bullet under
*Writing a test case* owns the mechanism.

**Settle first, because nothing in the tree answers it: a `Core` enum has no runtime value yet.** No
conformance case anywhere spells a `Core\…\Case`, `Core\Db\Isolation` has been registered since the
transaction slice with nothing that takes one, and § 7's `isolation` option is still unlanded — so what
`Column::type()` hands back is a decision that slice makes, not a shape to copy from a sibling.

**The material `columns()` needs is already in hand.** `crates/nvs-stdlib/src/db.rs:2604` already takes
the `Vec<PgColumn>` before the first row is read, `PgColumn`'s `name`/`type_oid`/`type_modifier` are
public and `column_type()` classifies, and `crate::instance::build` asserts slot arity — so a third
`ROWS` slot is a compile error at both build sites until both are edited, which is the cheap way to
find them.

**The driver's acceptance line still names `examples/queue.nvs`**, which is Stage 8's unlanded
`Core\Queue` (ADR 0084) and not a regression. **The CA is still not in git**; `nvs_host::tls`'s module
doc owns why.

## Next group

**`columns()` end to end, and the file set is `crates/nvs-stdlib/src/db.rs`,
`crates/nvs-stdlib/src/registry.rs` and `tests/conformance/core/`.** Nothing in `nvs-db` moves.

- [ ] **`Core\Db\Column` registers and `ROWS` gains its sixth member — one slice, because the coverage
      gate binds them.** Three readers per `docs/spec/01-core-library.md:1200` over three slots, with
      `crates/nvs-stdlib/src/db.rs:1056`'s `WRITE` as the template it copies exactly; the slot-name
      consts go beside `crates/nvs-stdlib/src/db.rs:248`, the class row beside
      `crates/nvs-stdlib/src/registry.rs:1413`, the symbols beside
      `crates/nvs-stdlib/src/db.rs:4029`. `ROWS` then takes a third slot filled at both build sites —
      `crates/nvs-stdlib/src/db.rs:2563` and `crates/nvs-stdlib/src/db.rs:2766` — off the columns
      `crates/nvs-stdlib/src/db.rs:2604` already holds, and `columns()` is a reader over it.
      **`nullable()` has no source**: a PostgreSQL `RowDescription` carries no NOT NULL flag and the
      catalog lookup that would is what § 9's table avoids, so answer `true` and say so on the member.
      ADR 0063, ADR 0067 § 9, spec § 18.
- [ ] **Three `.nvst` cases, type-level as every landed db case is.**
      `tests/conformance/core/db-rows-answers-the-types-the-results-table-names.nvst` is the shape to
      follow, and `crates/nvs-stdlib/src/db.rs:756`'s `ROWS` doc is what they pin: `columns()` answers
      `array<Core\Db\Column>`, `type()` an enum case and not a string, and a column list describes
      every column rather than only the ones a row happened to fill. The claim only a server can show
      — a column whose every row is NULL still has a type — goes in `examples/db.nvs` instead.

## Backlog

- Gap 2: only PostgreSQL runs a statement; the other four drivers are `crates/nvs-stdlib/src/db.rs`'s
  own known-gap list.
- The pool is Stages 3 to 7 of the goal — ADR 0067 § 13.
- `open` waits on a shape-parameter type — `docs/implementation-plan.md`'s *Open now*.
- Stage 8's `Core\Queue` is what the acceptance line has been naming — ADR 0084.
- `[context] modules` still lacks `nvs-runtime/src/object.rs`; this session also read
  `crates/nvs-stdlib/src/instance.rs` and `crates/nvs-stdlib/tests/conformance_coverage.rs`, neither
  of which the pack prints.
