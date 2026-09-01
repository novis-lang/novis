# Handoff

## State

**A compose server's certificate now verifies.** `tests/db/compose.yaml`'s `certs` service issues a
CA and a leaf signed by it (the old single `openssl req -x509` was `CA:TRUE` and rustls refused it as
an end-entity), `[db.<name>] tls_ca_file` names a PEM bundle, `nvs_config::db` resolves and
trust-checks it at boot, and `nvs_host::tls::anchors_from` parses it once per path and **replaces**
the compiled-in Mozilla set for that endpoint — that module's § *The trust anchors are compiled in*
owns why replace and not add.

**`examples/transaction.nvs` now fails inside the protocol**, which is the next group: `Core\Db\
Connection::execute: postgres ERROR: unnamed prepared statement does not exist (SQLSTATE 26000)` at
`examples/transaction.nvs:42`, i.e. the first statement after `BEGIN`. TLS, SCRAM and the startup
exchange are all behind it now.

**The CA is not in git.** It belongs to the `certs` volume and is remade with it, so `.gitignore`
holds `/tests/db/ca.crt` and `tools/loop.py` copies it out from the goal's new `[docker.copy]` table
during the bring-up. A checkout that has never brought the fixtures up has no such file, and because
a named-but-missing anchor bundle is a boot refusal, `nvs` run from the repo root will refuse until
it does — deliberate, and the alternative (ignoring an absent bundle) is the silent hole.

## Next group

**The extended-query state machine's statement lifetime — the file set is
`crates/nvs-db/src/pg.rs` and `crates/nvs-db/src/sql.rs`, plus one `.nvst` case.**

- [ ] **Find why the statement after `BEGIN` is bound to an unnamed statement that no longer
      exists** — `crates/nvs-db/src/pg.rs:644` is `query` and `crates/nvs-db/src/pg.rs:666`
      `execute_many`; `crates/nvs-db/src/sql.rs:356` is `StatementCache::prepare`, which answers
      `Prepared::Unnamed` whenever the capacity is `0`. `nvs.toml`'s `[db.main]` writes no
      `statement_cache`, so start by printing what `capacity_for` actually returns for it: an
      unnamed statement does not survive the `Sync` that ends a batch, so a *second* batch binding
      it is SQLSTATE 26000. ADR 0067 § 1 and § 4.
- [ ] **Pin it with a `-p nvs-db` case over the recorded exchange** — the playbook's bullet about
      `-p nvs-db` not being able to build a `PgConn` applies (`wire` is `Wire` at the default type),
      so the assertion is on `crates/nvs-db/src/sql.rs:356`'s answers across two batches rather than
      on a live connection.
- [ ] **Then re-run `target/debug/nvs.exe run examples/transaction.nvs` and `examples/db.nvs`** —
      `crates/nvs-stdlib/src/db.rs:1620` is `connect`, unchanged by this group. Stage 5's checks are
      what these two fixtures are.

## Backlog

- `columns()` on a `ColumnType` enum, the sixth `Rows` reader — ADR 0067 § 18.
- `open` waits on a shape-parameter type — ADR 0067 § 2.
- The pool, Stages 3 to 7 — ADR 0067 § 13.
- MySQL/MariaDB/SQL Server/SQLite drivers, each its own `tls_ca_file` wiring — ADR 0132 § 3.
- `[context] modules` still has no `nvs-config` pattern; this session read `tree.rs`, `resolve.rs`
  and `secret.rs` unguided. Add one.
