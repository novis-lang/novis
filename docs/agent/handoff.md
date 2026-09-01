# Handoff

## State

**Goal 5's Stage 0 is done and nothing else of the goal has landed.** ADR 0071's derive pass now runs over
both formats from one walk of a class: `nvs_types::derive::Format` is the only thing it branches on, the two
names are on `ATTRIBUTES` and matched nominally after `nvs_hir::resolve_ref`, and ADR 0067 § 9's type map is
`db_reachable` — refused at the declaration, never at the `queryAs<T>`. `ExprTypeTable::db_codec` holds the
row mapping; nothing generates `fromRow` from it yet, which is `derive.rs`'s own gap 2.

**`crates/nvs-db` still does not exist**, and neither does `tools/db-matrix.py`. `tests/db/compose.yaml`
does, and the driver preflights the Docker daemon before a session starts. The shape every session holds is
unchanged and lives in `docs/agent/loop-goal.md` § *The shape every session must hold*: a driver is
synchronous code over goal 2's parking stream (`crates/nvs-host/src/net.rs:130`), with `rustls`
(`crates/nvs-host/src/tls.rs:112`) layered on it.

**The goal's one ADR slot is unclaimed** — Stage 2 item 3, the driver crate's shape and its wire I/O. The
next free number was 0132 at this commit; re-check `git status --short docs/adr/` immediately before
creating the file.

## Next group

**The keystone's two prerequisites, then the seam itself** — Stage 2 in `docs/agent/loop-goal.md`, whose
own § *The harness this goal owes* says both harness files come before the first driver. One file set: the
ADR, `tools/db-matrix.py` and the new `crates/nvs-db`, none of which exist, over the two host types the
driver is written against.

- [ ] **ADR 0132 — the driver crate's shape and its wire I/O**, the goal's one pre-authorized slot: which
      protocol crate backs which driver, how TLS layers on the parking stream
      (`crates/nvs-host/src/tls.rs:112`), how a connection's busy state is tracked, and how the five
      drivers share code without a trait that flattens their differences. ADR 0067 specifies behaviour and
      deliberately not this. Add the row to `docs/adr/README.md`'s two tables and the bullet to
      `docs/adr/ground-rules.md`.
- [ ] **`python tools/db-matrix.py`** — runs ADR 0067's per-driver list against `tests/db/compose.yaml:1`'s
      five servers and prints one `<driver>: ok` line each. A harness, not a test: the assertions are
      `nvs-db`'s own.
- [ ] **`crates/nvs-db` exists, and a PostgreSQL connection is opened, TLS-wrapped and authenticated** over
      `crates/nvs-host/src/net.rs:150`'s `NvsTcp`, with `postgres-protocol` for the wire half. PostgreSQL
      first because its extended protocol pays nothing extra for a prepare. No async runtime — that is
      structural, per the goal's standing decisions.

## Backlog

- Spec § 6's `Core\Db` roster — `Codec`, `Row`, `Rows`, `Connection`, `Queryable` — is absent from
  `nvs_stdlib::registry`; `queryAs<T>` is written against it (docs/spec/01-core-library.md § 6).
- A `Format::Db` field still erases through JSON's `CodecTy`, so `bytes` and `Core\Time\Instant` land on
  `Opaque` (crates/nvs-types/src/derive.rs, gap 2).
- `orient.py`'s `[context] modules` patterns `crates/nvs-db/src/*.rs` and `crates/nvs-host/src/stream.rs`
  match nothing; the second wants to be `net.rs` and `tls.rs` (docs/agent/loop-goal.toml `[context]`).
- ADR 0071 § 1's table and `ATTRIBUTES` now agree on all thirteen rows; nothing else in the tree restates
  the roster (docs/adr/0071-derived-codecs.md § 1).
