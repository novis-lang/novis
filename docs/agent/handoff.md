# Handoff

## State

**MariaDB is a driver, not a leg that skips.** `crates/nvs-db/src/maria.rs` is new and holds
`MariaTarget`, `MariaConn::connect`, MariaDB's authentication roster and MariaDB's own ADR 0067 § 8 code
table. `MariaConn` carries MySQL's fields (wire, capabilities, § 1 cache, § 9 zone, § 7 depth) and the
statement, transaction and reset paths are two-line delegations into `crate::mysql`'s free functions —
one protocol is framed once. The module doc owns what is shared and what is not.

**What the split looks like in code**: `crate::mysql::Backend` is a `{name, kind_of}` descriptor carried
on the `Wire`, so a refusal anywhere reports `mariadb` and reads MariaDB's table without any reader
growing a parameter; `crate::mysql::Login` is the credential plus the plugin gate, so `authenticate` is
shared and the *roster* is each driver's own. That is the one place the two drivers touch, and
`maria.rs`'s module doc argues why it is not the MariaDB-as-a-flag design ADR 0067 rejects.

**`Cargo.toml` now takes `mysql_common` with `client_ed25519` and `client_parsec`** — pure Rust, which is
ADR 0051 § 4's first allowed outcome for the two plugins it names in advance. The manifest comment owns
that reasoning. Without the features the plugins still *parse*, so the gate would accept a plugin whose
handshake then fails with a Cargo message; `the_mariadb_auth_plugins_are_implemented_in_rust_or_refused_by_name`
in `maria.rs` is what holds the features in place.

**Nothing asserts MariaDB over a socket yet.** `crates/nvs-db/tests/handshake.rs` still has no
`mariadb()` fixture, so the matrix's MariaDB leg proves the container and not the driver — the next
group's first item, and the reason it is first.

**The driver's stage-2 acceptance check is unchanged and still open**:
`a_db_open_target_in_a_denied_range_fails` waits on `Core\Db::open`, blocked on a registry type for a
shape **parameter** (`nvs_stdlib::db` known gap 1) — a language-surface decision that wants its own ADR,
not a slice. Untouched this session.

**`orient.py` gaps: none**, though `[context] adrs` would have paid for ADR 0051 § 4 (it printed only as
a ground-rules bullet, and the slice turned on its two-outcome sentence).

## Next group

**MariaDB over a real socket, one file set: `crates/nvs-db/tests/handshake.rs`,
`crates/nvs-db/tests/pool_reuse.rs` and `crates/nvs-db/src/maria.rs`. `mysql()`/`mysql_connect_as` at
`crates/nvs-db/tests/handshake.rs:187` is the worked example for the first two.**

- [ ] **A `mariadb()` fixture and a handshake case** — the twin of `mysql()`, so the matrix's MariaDB leg
      asserts the driver rather than the container: § 3's upgrade completes, a wrong password is refused
      as `Permission`, and § 9's zone round-trips. `crates/nvs-db/tests/handshake.rs:187`,
      `crates/nvs-db/tests/handshake.rs:203`, `crates/nvs-db/src/maria.rs:@connect`.
- [ ] **`pool_reuse.rs` twinned a fourth way** — § 13's `COM_RESET_CONNECTION` over MariaDB, the same
      three assertions the MySQL leg makes. `crates/nvs-db/tests/pool_reuse.rs:1`,
      `crates/nvs-db/src/maria.rs:@reset`.
- [ ] **`mariadb_returning_is_available_and_mysqls_is_not`** — ADR 0067's `RETURNING`, which is the first
      thing MariaDB's statement path does that MySQL's cannot. `crates/nvs-db/src/maria.rs:@query`,
      `crates/nvs-db/src/sql.rs:89`.

## Backlog

- `Core\Db::open`'s shape-parameter registry type — `nvs_stdlib::db` known gap 1, wants an ADR.
- `MariaConn` is not yet reachable from `nvs-stdlib`: `db.rs`'s connect path has no MariaDB arm.
- `COM_STMT_BULK_EXECUTE` for `executeMany` on MariaDB — ADR 0067 § 4, `crates/nvs-db/src/maria.rs`.
- `TdsConn::connect` — ADR 0067 § 3's TLS-inside-TDS handshake, the largest remaining wire slice.
- SQLite's driver, the last of the five — `docs/plan/m8.md`.
