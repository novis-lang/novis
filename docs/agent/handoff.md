# Handoff

## State

**`Core\Db\Isolation` is registered** — `crates/nvs-stdlib/src/db.rs:487`, § 7's five cases as a
`CoreEnum` with its `EnumDoc`, in `registry::ENUMS`. Its values are declaration ordinals and
carry no comparison meaning; the constant's own doc says why, and `nvs_db::Isolation`
(`crates/nvs-db/src/conn.rs:203`) stays the authoritative half. No member takes it yet.

**`nvs.toml` now opens the database**: a `[db.main]` block for `tests/db/compose.yaml`'s
PostgreSQL on 15432, and `[app.capabilities.db] connect = ["main"]` for `examples/db.nvs` and
`examples/transaction.nvs` under their own entry blocks. That block's comment owns the reasoning.

**The Docker precondition is met and is not what blocks stage 5.** The fixture reaches the server
and fails at `invalid peer certificate: Other(OtherError(CaUsedAsEndEntity))`. Two separate things
owe that, and the next group is both: `tests/db/compose.yaml`'s `certs` service copies
`server.crt` to `ca.crt`, so the leaf *is* a CA certificate and no trust store can accept it as an
end-entity; and `nvs_host::tls` verifies against anchors compiled into the binary, with
`config_over` a seam nothing outside that module calls. Stages 6 and 9 are behind the same wall.

**`orient.py` printed nothing about any of this.** The goal's `[context] modules` has no
`nvs-config` or `nvs-host/src/tls.rs` pattern and nothing names `nvs.toml`, so a session whose
check fails on configuration re-derives the whole shape. Adding `nvs-config` to `modules` is the
cheap half.

## Next group

**The trust anchors, so a fixture can reach a compose server at all — the file set is
`tests/db/compose.yaml`, `crates/nvs-host/src/tls.rs`, `crates/nvs-config/src/tree.rs` and
`crates/nvs-db/src/pg.rs`.** Take them in this order; the first is what makes the second testable.

- [ ] **The compose `certs` service issues a CA and a leaf signed by it, not one self-signed
      certificate serving as both** — `tests/db/compose.yaml:46` is the service and
      `tests/db/compose.yaml:26` the comment that promises a CA. `openssl req -x509` makes a
      certificate with `CA:TRUE`, which is exactly what rustls refuses as an end-entity, so this is
      a second `openssl x509 -req` against the first and a `ca.crt` that is no longer a copy.
      Nothing in `crates/` changes for it, and the check is `target/debug/nvs.exe run
      examples/transaction.nvs` reporting a *trust* failure rather than `CaUsedAsEndEntity`.
- [ ] **An anchor bundle is a configured path, and `nvs_host::tls` grows the reader** —
      `crates/nvs-host/src/tls.rs:270` is `anchors()`, `crates/nvs-host/src/tls.rs:285` is
      `config_over`, the seam its own § *The trust anchors are compiled in* names; the config field
      goes beside the other endpoint keys on `crates/nvs-config/src/tree.rs:510`. **Prefer a
      per-`[db.<name>]` key over a process-wide one**: a trust set is a property of one server, and
      a global switch is the shape that later gets set for the wrong reason. That module's docs are
      explicit that a *program* choosing anchors is not on the table — this is the operator's file
      and nothing else.
- [ ] **`PgConn::connect` carries the bundle to the handshake** — `crates/nvs-db/src/pg.rs:374`
      (`PgTarget::resolve`, which already reads every other `Database` field) and the caller at
      `crates/nvs-stdlib/src/db.rs:1614`. § 3 has no spelling for turning TLS off and this adds
      none: what it adds is which anchors the verification runs against.

## Backlog

- § 7's `{isolation?, readOnly?}` bag reaching `PgConn::begin` — ADR 0067 § 7;
  `crates/nvs-stdlib/src/db.rs:437`, `crates/nvs-db/src/pg.rs:3049`. `ISOLATION` exists for it now.
- `{retries: n}`, outermost transactions only — ADR 0067 § 7's last paragraph.
- `Rows::columns()` on a `Core\ColumnType` enum — this module's known gap 5, in `db.rs`'s own doc.
- `Db::open` waits on a shape-parameter type — spec § 18, `Db\Settings`.
- A `use` alias does not reach a `catch` — the playbook bullet has the anchor.
- ADR 0132's driver shape for the other four drivers — the goal's stage 6.
