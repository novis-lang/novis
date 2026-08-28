# Handoff

## State

**Goal 5 of the parity program has just started; nothing of it has landed yet.** M4 and goals 1–4 reached
their whole acceptance lists and all five are now this goal's Stage 1 floor.

**`crates/nvs-db` does not exist yet.** The shape every session holds: a driver is **synchronous code over
goal 2's parking stream**, with `rustls` layered on the same stream for TLS. `sqlx`, `tokio-postgres` and
`tiberius` are not usable — structurally, because they need a runtime that spawns, not as a preference.
What is usable is the wire-protocol half of the ecosystem: `mysql_common`, `postgres-protocol`, our own
TDS, and `rusqlite` for SQLite under ADR 0051 § 4's one audited C exception.

**The Docker daemon must be reachable** and the driver preflighted it before this session started, so the
five containers in `tests/db/compose.yaml` are up. They are brought up once per run and the matrix is
memoized against `crates/nvs-db/**`, so a session that changes nothing there does not pay for it again.

## Next group

**`#[Db\Derive]` — Stage 0, and it is a catch-up rather than a feature.** The two names are deliberately
absent from `nvs_types::derive::ATTRIBUTES` today and
`crates/nvs-types/src/derive.rs:47` says why: "a closed list that names something with no pass behind it
is worse than a short one." It goes first because ADR 0067 § 6's `queryAs<T>` is written against it, and a
row mapping written without it is written twice.

One file set: `crates/nvs-types/src/derive.rs`, `crates/nvs-stdlib/src/registry.rs`.

- [ ] **`Core\Db\Derive` and `Core\Db\Field` join `ATTRIBUTES`**, matched nominally after
      `nvs_hir::resolve_ref` exactly as `#[Json\Derive]` is. Goal 1 closed that pass's three gaps; this is
      the same pass over a second format, so the shape to copy is `derive.rs`'s own — not a new one.
- [ ] **A field whose declared type has no column mapping is refused where it is declared**, not at the
      `queryAs<T>` that runs. That is the gap goal 1 closed for JSON and it must not be reintroduced here.
- [ ] **The two derives share one pass.** Two passes that agree today is the failure; `#[Json\Derive]` and
      `#[Db\Derive]` differ in their type map and in nothing else.

## Backlog

- Stage 2 carries this goal's one pre-authorized ADR slot — the driver crate's shape and its wire I/O.
  ADR 0067 specifies behaviour and deliberately does not specify this. **PostgreSQL first**, and no second
  driver until it is green end to end.
- `#[Test(db:)]`'s rolled-back transaction is ADR 0079 § 14 and waits for goal 6, with the test-server
  half it arrives beside.
- `Web\Migration` is deliberately blocked by ADR 0082 § 7. `nvs queue migrate` creates two tables ADR 0084
  specifies and is **not** a migration runner; a session generalising it has drifted into the blocked
  design.
