# Handoff

## State

**ADR 0067 § 3's `[db.<name>]` block resolves into a target.** `PgTarget::resolve` is
`crates/nvs-db/src/pg.rs:364`; it borrows the block rather than copying it, because a resolved target
has to outlive the `PgConn::connect` call that opens with it and `nvs_config`'s snapshot already
does — the function's own doc comment is the home of that argument and of why the password in
particular is not copied twice. `BlockError` beside it is the refusal as a *value*: it names the
field's own key and quotes what the block wrote, and `refusal("main")` is the one place a block's
name is joined to a field's fault. § 9's non-offset zone is `BlockError::TimeZone` rather than UTC —
the refusal `sql::time_zone_for`'s doc comment promised somebody makes, and that doc now points at
it. A block's `driver` reads through `Driver::from_config_name`, which is `matrix_name`'s roster and
differs from the matrix reader only in accepting the case a hand-written file is allowed.

**Nothing calls `resolve` yet, so the acceptance check is still red at `E0405` on
`Core\Db\Connection::query`** — the next group's first two items are what close it.

**There is no boot-time pass over `[db.*]`, and there cannot be one here**: `nvs-config` owns every
other boot error and cannot depend on `nvs-db` without closing the workspace cycle ADR 0132 § 1
names, so a block's refusal arrives at the first `Core\Db::connect` instead. Nothing about it is
per-request — the same block refuses the same way until an operator edits the file — and
`BlockError`'s doc says so. A genuine boot check needs a crate above both; none exists yet.

`Core\Db` still carries no `registry::CAPABILITIES` row, and the two capabilities are
`nvs_config::capability`'s `DbConnect`/`DbOpen` at `crates/nvs-config/src/capability.rs:51`.

**Manifest gaps, in `docs/agent/loop-goal.toml`:** `[context] adrs` names none of ADR 0067 §§ 2, 4,
7, 8 and no section of ADR 0132; `[context]` reaches nothing under `docs/spec/`, so § 18's class
tables come in only as the three lines around an anchor; `[context] modules` does not name
`nvs-config`.

## Next group

**The connection, which is what the acceptance check is failing on** — the file set is
`crates/nvs-stdlib/src/db.rs` and `crates/nvs-stdlib/src/registry.rs` on the stdlib side,
`docs/spec/01-core-library.md` § 18 for the signatures, and `crates/nvs-db/src/pg.rs` on the wire
side.

- [ ] **`Core\Db::connect` and `open`, with the capability row that makes them reachable** —
      `docs/spec/01-core-library.md:1144` for the two signatures, ADR 0067 §§ 2 and 3. The five
      edits go in `crates/nvs-stdlib/src/db.rs:101` (the `CLASS` rows and their cards) and
      `crates/nvs-stdlib/src/db.rs:285` (`address`), the capability row in
      `crates/nvs-stdlib/src/registry.rs:1440` against `crates/nvs-config/src/capability.rs:51`.
      The target comes from `crates/nvs-db/src/pg.rs:364` and the socket from
      `crates/nvs-db/src/pg.rs:560`. Decide first **where the address is pinned** — § 3 pre-approves
      a `connect`-named endpoint and `PgConn::connect` takes a `SocketAddr` nobody resolves yet —
      and **where a memoized connection lives for the request**.
- [ ] **`Core\Db\Connection` and `Queryable::query`, which is the member the check names** —
      `docs/spec/01-core-library.md:1156`, ADR 0067 §§ 4 and 6. The class shape to copy is
      `crates/nvs-stdlib/src/db.rs:131` (`IN_LIST`, the memberless carrier), the rows it wraps are
      `crates/nvs-db/src/pg.rs:560` and the statement path below it.
- [ ] **Three `.nvst` cases over whichever of those two landed**, under `tests/conformance/core/`,
      plus the line at `docs/reference/tools/20-config.md:374` that still says nothing opens a
      `[db]` block.

## Backlog

- The pool — ADR 0067 § 13, per core, reset as a boundary — is Stages 3 to 7 (`docs/plan/m8.md`).
- `BlockError` moves to `crates/nvs-db/src/conn.rs` when a second driver shares it, never copied.
- Nothing handshakes against `tests/db/compose.yaml`: self-signed, no anchor seam in `nvs_host::tls`.
- MySQL, MariaDB, SQL Server and SQLite drivers are unwritten (`docs/adr/0132`).
- `docs/agent/loop-goal.toml`'s `[context]` gaps, listed under *State* above.
