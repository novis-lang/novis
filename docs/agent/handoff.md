# Handoff

## State

**Stage 2's harness half is done, and the goal's one ADR slot is spent.**
[ADR 0132](../adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md) decides the four things
the first driver would otherwise decide in a commit message: a **borrowed sans-IO codec plus a state
machine we write** (`postgres-protocol`, `mysql_common`, `rusqlite`, hand-written TDS — every crate that
would have supplied a client needs a runtime that spawns); `NvsTls` **generalised over its transport**, so
SQL Server's TLS-inside-TDS handshake reaches the same session type and the same trust anchors; busy state
as a field on the **connection** and not on `NvsStream`, with a `Poisoned` wire **closed rather than
reset**; and five drivers as an **enum with one `match` per entry point**, no `Driver` trait. Read § 4 and
§ 5 before writing the first connection — they are the two that a driver silently violates.

**`python tools/db-matrix.py` exists**, next to the `tests/db/compose.yaml` that was already there. It
reads ports and credentials back out of the compose file (`docker compose config --format json`), so it
holds no copy of either, and hands one driver at a time to `cargo test -p nvs-db` through discrete
`NVS_DB_MATRIX_*` environment fields — never a DSN, because Novis has none. Its own module doc is the
contract's home. With no `crates/nvs-db` it prints `<driver>: n/a` and exits 2, deliberately not `ok`.

**`crates/nvs-db` still does not exist**, so the goal's three fixtures stay red at `E0405` and stage 6's
matrix check stays red — the ordinary state of this goal, not a regression.

## Next group

**The first driver, end to end** — Stage 2 items 3 and its two prerequisites in `docs/agent/loop-goal.md`,
all four specified by ADR 0132's §§ 1, 3 and 5. One file set: the new `crates/nvs-db`, the workspace
manifest, and the two `nvs-host` files the ADR changes.

- [ ] **`crates/nvs-db` exists** — the crate, its module doc carrying § 4's state machine and the
      `NVS_DB_MATRIX_*` contract, the `Connection` enum skeleton of § 5, and the workspace edges of § 1
      (`nvs-stdlib` depends on it, never the reverse; the workspace manifest's dependency table gains
      the path entry and the three wire crates). Anchors: `crates/nvs-stdlib/Cargo.toml:37`,
      `crates/nvs-host/Cargo.toml:12`.
- [ ] **`NvsTls` becomes generic over its transport** — ADR 0132 § 3, `NvsTls<T: Read + Write>` with
      `NvsTls<NvsTcp>` kept as the alias `Core\Http\Client` and `Core\Mail` already use, so the TDS
      framer can be an ordinary adapter later. Anchors: `crates/nvs-host/src/tls.rs:112`,
      `crates/nvs-host/src/tls.rs:145`.
- [ ] **A PostgreSQL connection is opened, TLS-wrapped and authenticated** — loop-goal Stage 2 item 3
      over `NvsTcp`: `SSLRequest`, the `NvsTls` upgrade, then SCRAM through `postgres-protocol`, with the
      wire state at `Idle` when the handshake returns. Anchors: `crates/nvs-host/src/net.rs:130`,
      `crates/nvs-host/src/tls.rs:145`.

## Backlog

- Stage 2 items 4–7: named/settings connections, `db.connect`/`db.open`, the `LOCAL INFILE` refusal and
  the forced UTF-8 charset — `docs/agent/loop-goal.md`.
- `libsqlite3-sys` gains its `C_DEPENDENCIES` entry when the SQLite driver lands — `tools/gen-attribution.py`,
  under ADR 0132 § 2.
- The `[context] modules` manifest named `crates/nvs-host/src/stream.rs`, which never existed; it now names
  `net.rs`, `tls.rs` and `blocking.rs`. `crates/nvs-db/src/*.rs` still warns until the crate exists —
  `docs/agent/loop-goal.toml`.
- `orient.py` printed no `Cargo.toml` or crate-manifest window; the next session adds a `[context]` selector
  for it rather than grepping — `docs/agent/loop-goal.toml`.
