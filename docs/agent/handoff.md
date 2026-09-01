# Handoff

## State

**ADR 0067 § 2's `Core\Db::connect` is on the roster and reaches a server.** The body is
`crates/nvs-stdlib/src/db.rs:@nvs_core_db_connect`: the `db.connect` grant is asked *first*, so a
program with no grant learns nothing about which blocks a deployment wrote; then the block comes
from the boot snapshot and never through `Request::get`, because a credential read through a table
`Core\Config::set` can write to would let a request choose its own server. § 3's pre-approval is
`address_of` in the same file — an operator-written endpoint is resolved and pinned and is
deliberately *not* asked about ADR 0058 § 3's denied ranges, which is where every database on a
`10/8` estate lives.

**A connection is request-scoped and there is no reuse across requests yet.**
`nvs_runtime::Ctx::hold_open_connection` (`crates/nvs-runtime/src/ctx.rs`) is the table and the one
home of why: a `Core` handle is a key into a request-owned table, and § 13's per-core pool is what
would let a connection outlive its request — behind that section's reset and not before. The
`HeldConnection` trait beside it is the seam ADR 0132 § 1's crate edge forces: `nvs-db` depends on
`nvs-runtime`, so a field typed `nvs_db::Connection` would close a cycle.

**`Core\Db::open` is blocked on a type, not on a body.** § 18 writes
`open(Db\Settings $settings, {shared?: bool})` and `Db\Settings` is a discriminated union of two
shapes; `registry::CoreTy` has no variant for a shape **parameter** at all — `Options` is a trailing
bag, flattened one ABI argument per option — and no row in the crate has ever declared one. Adding
that variant decides how every future shape parameter is passed, so it is a slice of its own.
`crates/nvs-stdlib/src/db.rs`'s known gaps own this and the other three.

**The acceptance check is still red, one member further along**: `examples/transaction.nvs` now
fails on `Core\Db\Connection` having no `query`, which is the next group's first item. `Connection`
is a two-slot handle on `registry`'s roster until `Queryable`'s members land.

## Next group

**The connection's own surface — the file set is `crates/nvs-stdlib/src/db.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-db/src/pg.rs` and `docs/spec/01-core-library.md`
§ 18.**

- [ ] **`Core\Db\Connection::query`, which is the member the acceptance check names** —
      `docs/spec/01-core-library.md:1156` for the signature, ADR 0067 §§ 4 and 6. The rows go on
      `crates/nvs-stdlib/src/db.rs:145` (`CONNECTION`, which stops being a handle and leaves
      `crates/nvs-stdlib/src/registry.rs:3466`'s `HANDLES` list), the receiver's connection comes
      back out of the request table — `crates/nvs-runtime/src/ctx.rs:2981` is `hold_open_connection`
      and this slice adds the `open_connection_mut` beside it that downcasts through
      `HeldConnection::as_any_mut`. The wire is `crates/nvs-db/src/pg.rs:560` onwards; § 5's
      rewriter is `crates/nvs-db/src/sql.rs`.
- [ ] **`Core\Db\Rows` and `Core\Db\Row`, which is what `query` answers with** —
      `docs/spec/01-core-library.md:1183` for both tables, ADR 0067 § 6 for the coercion rule.
      `crates/nvs-db/src/pg.rs`'s `PgRow`/`PgScalar` are already the decode; what is missing is the
      `Core` side, and the five `PgScalar` variants are the ones only `nvs-stdlib` can build.
- [ ] **Three `.nvst` cases over `query`**, under `tests/conformance/core/`, on the pattern
      `tests/conformance/core/db-connect-judges-its-options-before-it-consults-the-grant.nvst:1`
      sets: a refusal a case can reach without a server, and the row they are counted against is
      `crates/nvs-stdlib/src/db.rs:145`. Run them with `target/debug/nvs.exe test <path>`, never
      `tools/try.py` — the playbook bullet says why.

## Backlog

- `Core\Db::open` needs a `CoreTy` for a shape parameter — `crates/nvs-stdlib/src/db.rs`'s gap 1.
- `Db\DbError`/`Db\RolledBack` are not in spec § 10's tree, so every refusal here is a bare
  `RuntimeError` — same file, gap 4.
- ADR 0067 § 13's per-core pool and its reset — `docs/plan/m8.md`, stages 3 to 7.
- Four of five drivers have no connect path; a non-PostgreSQL block is refused by name.
- `[context] adrs` in `docs/agent/loop-goal.toml` still names no section of ADR 0132 and none of
  0067 §§ 4, 7, 8; `[context] modules` still does not name `nvs-config`.
