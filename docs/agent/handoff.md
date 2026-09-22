# Handoff

## State

Goal `core-db-transaction-and-1-more` is met and **its floor is green**. Both DONE claims before this
one fell to a stale perf figure, and the cause was one commit ahead of them: the `html` literal's new
section moved `docs/reference/lang/10-programs.md` and `30-expressions.md`, which stales every feature
keyed on a chapter. Those 22 figures are re-measured, and three programs got *cheaper* rather than
dearer — `lang:programs/a-complete-program-annotated` allocates six times a round where it allocated
eight, `lang:expressions/arrays-in-expressions` six where it was seven, `lang:expressions/object-literals`
once where it was twice.

**MySQL keeps an absent key an absence.** `nvs_db::Answer::Done::last_id` and `MySqlRows::last_id`
carry `Option<u64>` from the status packet down, so the driver no longer mints a `0` for
`nvs_stdlib::db::execute` to filter back out — `mysql_common` maps the protocol's own `0` to `None`
already, and the member now means one thing on every driver. The observable answer is unchanged.
`Core\Db\Write::lastId`'s reference card says what each of the four drivers reads the key out of and
that a statement which inserted no row answers `null`; that card edit staled all 57 `Core\Db*`
figures, which are re-measured and moved only in the third decimal.

Nothing in the tree is stale: `python tools/verify.py` is 14 of 14 green, `--doc` green, and
`owners.py --closes` and `playbook.py --closes` name nothing for this goal.

## Next group

**Stage: what a write reports on the two drivers nobody has pinned** — one file set:
`crates/nvs-stdlib/src/db/execute.rs`, `crates/nvs-db/src/pg.rs`.

- [ ] **Pin that SQL Server's `lastId` is `null` and that this is an answer** — the doc comment at
      `crates/nvs-stdlib/src/db/execute.rs:840` says the token stream carries no generated key and
      `nvs_db::tds::TdsRows` has no `last_id` at all, and no test asserts what a caller then reads.
      `rule:core-classes/db-statement-members`.
- [ ] **Pin the stdlib arm of PostgreSQL's key, not just the driver's** — `crates/nvs-db/src/pg.rs:2816`
      is pinned on the driver side by `a_statement_that_returned_no_key_has_no_last_id`, while the arm
      that carries it into a `Written` is `crates/nvs-stdlib/src/db/execute.rs:786` and nothing asks it
      for a statement with no `returning` clause. `rule:core-classes/db-statement-members`.

## Backlog

- A `--stale` selector on `tools/dossier.py` would answer the currency question in one call; today it
  takes one `--gate --group` per group. Nobody owns it.
- `benches/members/lang/expressions/{assignment,calls,closures}.nvs` carry `known-gap:` markers for
  allocations a `callable` call and a `uint` array subscript cost — owned by
  `crates/nvs-runtime/src/lib.rs` and `crates/nvs-ir/src/lib.rs`.
- `Core\Queue`'s MySQL insert fatals when the OK packet carries no key
  (`crates/nvs-stdlib/src/queue.rs:3959`); `schema` declares the column that makes it impossible, and
  nothing pins the fatal.
