# Handoff

## State

**Goal 58 — every Core\\Db and Core\\Queue member answers on all five drivers — has just started; nothing of it has landed yet.** Goal `m7-server-surface`'s whole list is this goal's Stage 1 floor.

Settled before the first session, and not re-decided (the goal's § *Standing decisions*):
- Stage 2's record is the design, and it is one new record with no number named in advance.
- A driver that cannot park a cursor gets a **recorded refusal naming the driver**, never a buffer.
  SQLite is the likely one, because its rows are materialized through an `Arc<Mutex<Connection>>`
  (`crates/nvs-db/src/sqlite.rs:33-40`).
- `serverVersion` is what the handshake already sent, stored on the connection with no round trip.
- A unique key reads nulls as distinct on every backend, and SQL Server spells it as a filtered index.
- `queryAs<T>`'s remaining refusal becomes `E0806`; no new code is spent.

Already closed, and not re-worked:
- `queryAs`'s list, no-derive and unfilled-constructor refusals are already `E0806`
  (`crates/nvs-types/src/derive.rs:713`).
- The queue's secret refusal is pinned (`tests/conformance/reject/queue-push-refuses-a-secret.nvst`).
- The worker's SQLite arm exists (`crates/nvs-cli/src/worker.rs:331`).

## Next group

**Stage 0: the catch-up** — one file set: the module docs of `db/mod.rs`, `queue.rs`, `derive.rs` and `worker.rs`, the part-two outstanding list, and one playbook bullet.

- [ ] **Owner tags that name retired goals become `m8-db-queue`**:
      - `crates/nvs-stdlib/src/db/mod.rs:290` and `:307`, and `crates/nvs-stdlib/src/queue.rs:109`
        (`gap-zero`);
      - `crates/nvs-stdlib/src/queue.rs:85` (`unowned-sweep`);
      - `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt:35` (`carried-gaps`).

      The M8-tagged gaps at `crates/nvs-types/src/derive.rs:71` and `:83`, and at
      `crates/nvs-cli/src/worker.rs:101`, take the same tag. `rule:core-api/tier-roster` governs the
      owner column.
- [ ] **Rewrite the stale gaps whole** (AGENTS.md rule 6):
      - `crates/nvs-stdlib/src/queue.rs:101` gap 4 still says the worker has no SQLite arm, but it
        has one at `crates/nvs-cli/src/worker.rs:455`;
      - `crates/nvs-stdlib/src/db/mod.rs:291` gap 4: only the `Opaque`-field refusal is still per row;
      - `crates/nvs-stdlib/src/queue.rs:64` gap 1: only `limits` and `grants` are left;
      - `crates/nvs-types/src/derive.rs:72` gap 2: `Core\Db\Row` exists, and
        `crates/nvs-stdlib/src/db/row.rs:122` is the walk.
- [ ] **`docs/agent/playbook.md:3880`** says queue statements exist "for PostgreSQL and MySQL only".
      Correct it to four backends, per `crates/nvs-stdlib/src/queue.rs:2368`.

## Backlog

- Stage 2, the record, which is prose only: `docs/decisions/`, and `docs/rules/core-classes/` for
  `db-streaming`, `schema-plan` and `queue-storage-is-a-table`.
- Stages 3–4, `stream` on four drivers: `crates/nvs-db/src/{mysql,maria,sqlite}.rs`,
  `crates/nvs-db/src/tds/rows.rs` and `crates/nvs-stdlib/src/db/stream.rs`, plus a new
  `crates/nvs-stdlib/tests/db_stream.rs` joining `tools/db-matrix.py` `SUITES`.
- Stage 5, `streamAs` and `serverVersion`: `crates/nvs-stdlib/src/db/registry.rs`,
  `crates/nvs-stdlib/tests/spec_registry_coverage.rs:951`, and each driver's handshake.
- Stage 6, the row decoders: `crates/nvs-types/src/derive.rs`, `crates/nvs-stdlib/src/db/row.rs`,
  `crates/nvs-stdlib/src/db/column.rs` and `crates/nvs-stdlib/src/json.rs`.
- Stages 7–8, the queue: `crates/nvs-stdlib/src/queue.rs`, `crates/nvs-cli/src/worker.rs`,
  `crates/nvs-db/src/ddl.rs`, `crates/nvs-db/src/catalog.rs` and `crates/nvs-types/src/expr/isolate.rs`.
  Stage 9, the socket and CI legs: `tests/db/compose.yaml`, `tools/db-matrix.py`,
  `.github/workflows/ci.yml` and `tools/ci-changes.py`.
- When this goal's last check goes green, the driver takes goal `m8-stdlib-depth`.
